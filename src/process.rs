//! Spawn cargo subcommands with n2-style streaming output.
//!
//! What we do borrow from n2:
//! - stdout and stderr share one pipe, so the callback sees them in time order
//! - bytes are delivered to `output_cb` as they arrive, not after exit
//! - a captured child runs in its own process group so Ctrl-C can kill cargo
//!   *and* the rustc/link grandchildren it started
//!
//! A verbose child keeps our own process group: it shares the terminal with
//! us, so the terminal's Ctrl-C reaches it and everything it started without
//! us having to forward anything. The price is that a signal sent to us
//! alone only stops that child, not the children it started.
//!
//! What a platform makes of all that lives in `process_unix.rs` and
//! `process_windows.rs`.
//!
//! <https://github.com/evmar/n2>

use std::ffi::{OsStr, OsString};
use std::io::{self, IsTerminal, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Once, PoisonError};

#[cfg_attr(unix, path = "process_unix.rs")]
#[cfg_attr(windows, path = "process_windows.rs")]
mod platform;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Success,
    Interrupted,
    /// The command failed. Carries the child's exit code, or `128 + signal`
    /// when it was killed by a signal, following shell convention.
    Failure(i32),
}

impl Termination {
    /// The exit code that reports this outcome to the shell.
    #[inline]
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            Termination::Success => 0,
            Termination::Interrupted => 130,
            Termination::Failure(code) => code,
        }
    }
}

static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CHILDREN: Mutex<Vec<Tracked>> = Mutex::new(Vec::new());

/// A child that may still be running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tracked {
    pid: u32,
    /// False for verbose children, which share our process group.
    own_group: bool,
}

/// What to send a child that is still running.
#[derive(Debug, Clone, Copy)]
enum Signal {
    /// Ask the child to stop.
    Interrupt,
    /// Take the child down.
    Kill,
}

static HANDLER: Once = Once::new();

#[inline]
pub fn was_interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Locate the cargo binary.
///
/// Cargo sets `CARGO` to the path of the cargo binary running this
/// subcommand, which both avoids a PATH lookup and guarantees the same
/// toolchain is used for the spawned commands. Fall back to `cargo` on
/// PATH when invoked without cargo (e.g. running the binary directly).
#[inline]
#[must_use]
pub fn cargo_bin() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Catch SIGINT, SIGTERM, and SIGHUP (and Windows Ctrl-C) and kill registered
/// process groups.
///
/// Safe to call more than once; only the first call installs the handler.
pub fn install_interrupt_handler() {
    HANDLER.call_once(|| {
        let _ = ctrlc::set_handler(handle_interrupt);
    });
}

/// The first interrupt asks the children to stop; a later one kills them.
///
/// A verbose child shares our process group, so a signal meant for the group
/// would hit us as well: it is signalled on its own. The terminal's Ctrl-C
/// reaches the whole group, so such a child may see the interrupt twice.
fn handle_interrupt() {
    let already = INTERRUPTED.swap(true, Ordering::SeqCst);
    let signal = if already {
        Signal::Kill
    } else {
        Signal::Interrupt
    };
    let children = CHILDREN.lock().unwrap_or_else(PoisonError::into_inner);
    for child in children.iter().copied() {
        platform::signal_child(child, signal);
    }
}

struct TrackedGuard {
    child: Tracked,
}

impl Drop for TrackedGuard {
    fn drop(&mut self) {
        let mut children = CHILDREN.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(i) = children.iter().position(|&c| c == self.child) {
            children.swap_remove(i);
        }
    }
}

impl Tracked {
    fn track(self) -> TrackedGuard {
        CHILDREN
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(self);
        TrackedGuard { child: self }
    }
}

/// Run `program` with `args`, streaming merged stdout+stderr into `output_cb`.
///
/// When `verbose` is set, the child inherits this process's stdio and
/// `output_cb` is not used.
///
/// When `color` is set, a child writing to a pipe is told to colorize anyway:
/// the output is captured rather than shown as it arrives, and the console
/// that reprints it later is drawing on a terminal, where color is what cargo
/// would have used on its own. A plain console, or a pipe all the way through,
/// leaves cargo to detect for itself.
pub fn run_command(
    program: impl AsRef<OsStr>,
    args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    verbose: bool,
    color: bool,
    mut output_cb: impl FnMut(&[u8]),
) -> io::Result<Termination> {
    if was_interrupted() {
        return Ok(Termination::Interrupted);
    }

    let mut cmd = Command::new(program);
    cmd.args(args);

    // A verbose child uses the tty, so it must stay in our foreground group
    // or SIGTTIN/SIGTTOU would stop it.
    let own_group = !verbose;
    if own_group {
        platform::configure_process_group(&mut cmd);
    }

    let pipe_reader = if verbose {
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        None
    } else {
        let (reader, writer) = os_pipe::pipe()?;
        if color && io::stdout().is_terminal() && std::env::var_os("CARGO_TERM_COLOR").is_none() {
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
    let tracked = Tracked { pid, own_group };
    let _guard = tracked.track();
    // Deliver an interrupt that arrived before the child was tracked.
    if was_interrupted() {
        platform::signal_child(tracked, Signal::Interrupt);
    }

    if let Some(mut reader) = pipe_reader {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output_cb(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => {
                    abort_child(&mut child, pid, reader);
                    return Err(e);
                }
            }
        }
    }

    let status = child.wait()?;
    Ok(termination_from_status(status))
}

/// Close the pipe so writers get EPIPE, then SIGKILL the group: a child that
/// ignores SIGINT would keep `wait` from returning.
fn abort_child(child: &mut Child, pid: u32, reader: os_pipe::PipeReader) {
    drop(reader);
    platform::signal_child(
        Tracked {
            pid,
            own_group: true,
        },
        Signal::Kill,
    );
    let _ = child.wait();
}

fn termination_from_status(status: ExitStatus) -> Termination {
    if was_interrupted() {
        return Termination::Interrupted;
    }
    if status.success() {
        return Termination::Success;
    }
    if platform::signaled_interrupt(status) {
        INTERRUPTED.store(true, Ordering::SeqCst);
        return Termination::Interrupted;
    }
    Termination::Failure(platform::exit_code(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Run a shell command and collect what it wrote.
    pub(super) fn run_shell(script: &str) -> (Termination, Vec<u8>) {
        let (program, args) = platform::shell_command(script);
        let mut output = Vec::new();
        let term = run_command(program, args, false, false, |buf| {
            output.extend_from_slice(buf)
        })
        .expect("spawn shell command");
        (term, output)
    }

    #[test]
    fn reports_failure() {
        let (term, _) = run_shell("exit 7");
        assert_eq!(term, Termination::Failure(7));
    }

    #[test]
    fn maps_outcomes_to_exit_codes() {
        assert_eq!(Termination::Success.exit_code(), 0);
        assert_eq!(Termination::Interrupted.exit_code(), 130);
        assert_eq!(Termination::Failure(101).exit_code(), 101);
    }
}
