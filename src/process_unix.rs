//! The Unix side of running a child: the process group it leads, the signals
//! it gets, and the exit code it leaves behind.

use super::{Command, ExitStatus, Signal, Tracked};

/// The shell-facing exit code of a failed child. A child killed by a signal
/// has no exit code, so follow the shell convention of `128 + signal`.
pub fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match (status.code(), status.signal()) {
        (Some(code), _) => code,
        (None, Some(signal)) => 128 + signal,
        (None, None) => 1,
    }
}

/// Put the child in a process group of its own, so that a signal meant for
/// it reaches the grandchildren it started as well.
pub fn configure_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

/// Signal the child's group if it leads one, otherwise just the child.
///
/// A verbose child shares our group, and a signal meant for the group would
/// reach us as well, so it is signalled on its own.
pub fn signal_child(child: Tracked, signal: Signal) {
    let sig = match signal {
        Signal::Interrupt => libc::SIGINT,
        Signal::Kill => libc::SIGKILL,
    };
    let Ok(pid) = libc::pid_t::try_from(child.pid) else {
        return;
    };
    // A negative pid addresses the process group the child leads.
    let target = if child.own_group { -pid } else { pid };
    unsafe {
        libc::kill(target, sig);
    }
}

/// Whether the child was stopped by an interrupt.
pub fn signaled_interrupt(status: ExitStatus) -> bool {
    use std::os::unix::process::ExitStatusExt;
    matches!(status.signal(), Some(libc::SIGINT | libc::SIGTERM))
}

/// The shell a command runs in.
#[cfg(test)]
pub fn shell_command(script: &str) -> (&'static str, Vec<&str>) {
    ("/bin/sh", vec!["-c", script])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::tests::run_shell;
    use crate::process::{abort_child, run_command, Termination};
    use std::io::Read;
    use std::process::{Command, Stdio};

    /// The process group of the shell, as the shell sees it.
    const PGID_OF_SHELL: &str = "\"$(ps -o pgid= -p $$ | tr -d ' ')\"";

    #[test]
    fn captures_stdout() {
        let (term, output) = run_shell("printf 'hello\\n'");
        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("hello"), "{text:?}");
    }

    #[test]
    fn merges_stderr_into_the_same_stream() {
        let (term, output) = run_shell("printf 'out\\n'; printf 'err\\n' >&2; printf 'out2\\n'");
        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("out"), "{text:?}");
        assert!(text.contains("err"), "{text:?}");
        assert!(text.contains("out2"), "{text:?}");
    }

    #[test]
    fn reports_signal_death_as_128_plus_signal() {
        let (term, _) = run_shell("kill -9 $$");
        assert_eq!(term, Termination::Failure(137));
    }

    /// Outside our foreground group, a tty read would stop it with SIGTTIN.
    #[test]
    fn verbose_child_stays_in_our_process_group() {
        let ours = unsafe { libc::getpgrp() };
        let script = format!("[ {PGID_OF_SHELL} = {ours} ]");
        let term = run_command("/bin/sh", ["-c", &script], true, false, |_| {})
            .expect("spawn shell command");
        assert_eq!(
            term,
            Termination::Success,
            "child left process group {ours}"
        );
    }

    /// Its own group lets an interrupt reach cargo's grandchildren.
    #[test]
    fn captured_child_leads_its_own_process_group() {
        let (term, output) = run_shell(&format!("[ {PGID_OF_SHELL} = $$ ]"));
        assert_eq!(
            term,
            Termination::Success,
            "{:?}",
            String::from_utf8_lossy(&output)
        );
    }

    #[test]
    fn abort_child_does_not_hang_on_a_stubborn_writer() {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let (mut reader, writer) = os_pipe::pipe().expect("create pipe");
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "trap '' INT; exec yes"])
            .stdin(Stdio::null())
            .stdout(writer.try_clone().expect("clone pipe writer"))
            .stderr(writer.try_clone().expect("clone pipe writer"));
        configure_process_group(&mut cmd);
        let mut child = cmd.spawn().expect("spawn shell command");
        drop(cmd);
        let pid = child.id();

        // Wait until the SIGINT trap is in place.
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).expect("read child output");

        // The parent keeps a writer of its own, so closing the reader hands
        // the child no EPIPE: only the kill can end it.
        let _keep_alive = writer;

        let (done_tx, done_rx) = mpsc::channel();
        thread::spawn(move || {
            abort_child(&mut child, pid, reader);
            let _ = done_tx.send(());
        });
        if done_rx.recv_timeout(Duration::from_secs(5)).is_err() {
            if let Ok(group) = libc::pid_t::try_from(pid) {
                unsafe {
                    libc::kill(-group, libc::SIGKILL);
                }
            }
            panic!("abort_child hung waiting for a child that ignores SIGINT");
        }
    }

    #[test]
    fn interleaves_stdout_and_stderr_in_time_order() {
        let (term, output) = run_shell("printf A; printf B >&2; printf C");
        assert_eq!(term, Termination::Success);
        assert_eq!(output, b"ABC");
    }
}
