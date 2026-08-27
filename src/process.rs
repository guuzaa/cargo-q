//! Spawn cargo subcommands with n2-style streaming output.
//!
//! What we do borrow from n2:
//! - stdout and stderr share one pipe, so the callback sees them in time order
//! - bytes are delivered to `output_cb` as they arrive, not after exit
//! - each child runs in its own process group so Ctrl-C can kill cargo *and*
//!   the rustc/link grandchildren it started

use std::ffi::OsStr;
use std::io::{self, IsTerminal, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Success,
    Interrupted,
    Failure,
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static HANDLER: Once = Once::new();

pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Catch SIGINT/SIGTERM (and Windows Ctrl-C) and kill registered process groups.
///
/// Safe to call more than once; only the first call installs the handler.
pub fn install_interrupt_handler() {
    HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(handle_interrupt);
    });
}

fn handle_interrupt() {
    let already = INTERRUPTED.swap(true, Ordering::SeqCst);
    let pids = PIDS.lock().unwrap_or_else(|e| e.into_inner());
    for pid in pids.iter().copied() {
        kill_process_group(pid, already);
    }
}

struct PidGuard {
    pid: u32,
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        let mut pids = PIDS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = pids.iter().position(|&p| p == self.pid) {
            pids.swap_remove(i);
        }
    }
}

fn register_pid(pid: u32) -> PidGuard {
    PIDS.lock().unwrap_or_else(|e| e.into_inner()).push(pid);
    PidGuard { pid }
}

/// Run `program` with `args`, streaming merged stdout+stderr into `output_cb`.
///
/// When `verbose` is set, the child inherits this process's stdio and
/// `output_cb` is not used.
pub fn run_command(
    program: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    verbose: bool,
    mut output_cb: impl FnMut(&[u8]),
) -> io::Result<Termination> {
    if was_interrupted() {
        return Ok(Termination::Interrupted);
    }

    let mut cmd = Command::new(program);
    cmd.args(args);
    configure_process_group(&mut cmd);

    let mut pipe_reader = if verbose {
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        None
    } else {
        let (reader, writer) = os_pipe::pipe()?;
        if io::stdout().is_terminal() && std::env::var_os("CARGO_TERM_COLOR").is_none() {
            cmd.env("CARGO_TERM_COLOR", "always");
        }
        cmd.stdin(Stdio::null())
            .stdout(writer.try_clone()?)
            .stderr(writer);
        Some(reader)
    };

    let mut child = cmd.spawn()?;
    drop(cmd); // release the writer halves held by `cmd`, or reader.read() never sees EOF

    let pid = child.id();
    let _guard = register_pid(pid);
    if was_interrupted() {
        kill_process_group(pid, false);
    }

    if let Some(reader) = pipe_reader.as_mut() {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output_cb(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    kill_process_group(pid, false);
                    let _ = child.wait();
                    return Err(e);
                }
            }
        }
    }

    let status = child.wait()?;
    Ok(termination_from_status(status))
}

fn termination_from_status(status: ExitStatus) -> Termination {
    if was_interrupted() {
        return Termination::Interrupted;
    }
    if status.success() {
        Termination::Success
    } else if signaled_interrupt(status) {
        INTERRUPTED.store(true, Ordering::SeqCst);
        Termination::Interrupted
    } else {
        Termination::Failure
    }
}

#[cfg(unix)]
fn configure_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(windows)]
fn configure_process_group(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_group(_cmd: &mut Command) {}

#[cfg(unix)]
fn kill_process_group(pid: u32, escalate: bool) {
    let sig = if escalate {
        libc::SIGKILL
    } else {
        libc::SIGINT
    };
    unsafe {
        libc::kill(-(pid as i32), sig);
    }
}

#[cfg(windows)]
fn kill_process_group(pid: u32, escalate: bool) {
    unsafe {
        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
        if escalate {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn kill_process_group(_pid: u32, _escalate: bool) {}

#[cfg(unix)]
fn signaled_interrupt(status: ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    matches!(status.signal(), Some(libc::SIGINT | libc::SIGTERM))
}

#[cfg(not(unix))]
fn signaled_interrupt(_status: ExitStatus) -> bool {
    false
}

#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CTRL_BREAK_EVENT: u32 = 1;
#[cfg(windows)]
const PROCESS_TERMINATE: u32 = 0x0001;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GenerateConsoleCtrlEvent(event: u32, process_group_id: u32) -> i32;
    fn OpenProcess(
        desired_access: u32,
        inherit_handle: i32,
        process_id: u32,
    ) -> *mut std::ffi::c_void;
    fn TerminateProcess(handle: *mut std::ffi::c_void, exit_code: u32) -> i32;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn shell_command(script: &str) -> (&'static str, Vec<&str>) {
        ("/bin/sh", vec!["-c", script])
    }

    #[cfg(windows)]
    fn shell_command(script: &str) -> (&'static str, Vec<&str>) {
        ("cmd", vec!["/C", script])
    }

    fn run_shell(script: &str) -> (Termination, Vec<u8>) {
        let (program, args) = shell_command(script);
        let mut output = Vec::new();
        let term = run_command(program, args, false, |buf| output.extend_from_slice(buf))
            .expect("spawn shell command");
        (term, output)
    }

    #[test]
    fn captures_stdout() {
        #[cfg(unix)]
        let (term, output) = run_shell("printf 'hello\\n'");
        #[cfg(windows)]
        let (term, output) = run_shell("echo hello");

        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("hello"), "{text:?}");
    }

    #[test]
    fn merges_stderr_into_the_same_stream() {
        #[cfg(unix)]
        let (term, output) = run_shell("printf 'out\\n'; printf 'err\\n' >&2; printf 'out2\\n'");
        #[cfg(windows)]
        let (term, output) = run_shell("echo out& echo err 1>&2& echo out2");

        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("out"), "{text:?}");
        assert!(text.contains("err"), "{text:?}");
        assert!(text.contains("out2"), "{text:?}");
    }

    #[test]
    fn reports_failure() {
        #[cfg(unix)]
        let (term, _) = run_shell("exit 7");
        #[cfg(windows)]
        let (term, _) = run_shell("exit 7");

        assert_eq!(term, Termination::Failure);
    }

    #[cfg(unix)]
    #[test]
    fn interleaves_stdout_and_stderr_in_time_order() {
        let (term, output) = run_shell("printf A; printf B >&2; printf C");
        assert_eq!(term, Termination::Success);
        assert_eq!(output, b"ABC");
    }
}
