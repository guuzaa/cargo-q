//! The Windows side of running a child: the process group it leads, the
//! console events it gets, and the exit code it leaves behind.

use super::{Command, ExitStatus, Signal, Tracked};
use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Console::{GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, CREATE_NEW_PROCESS_GROUP, PROCESS_TERMINATE,
};

/// A Windows child has no signal to die from, so it only leaves an exit code.
pub fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

/// Put the child in a process group of its own, so that a console event meant
/// for it reaches the grandchildren it started as well.
pub fn configure_process_group(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// CTRL_BREAK only targets process groups, so a child in ours is terminated.
pub fn signal_child(child: Tracked, signal: Signal) {
    if child.own_group {
        unsafe {
            GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child.pid);
        }
    }
    // A child in our own group cannot be signalled as a group without
    // hitting us as well, so it is terminated instead.
    if matches!(signal, Signal::Kill) || !child.own_group {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, child.pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }
}

/// Windows has no signal to interrupt a child with.
pub fn signaled_interrupt(_status: ExitStatus) -> bool {
    false
}

/// The shell a command runs in.
#[cfg(test)]
pub fn shell_command(script: &str) -> (&'static str, Vec<&str>) {
    ("cmd", vec!["/C", script])
}

#[cfg(test)]
mod tests {
    use crate::process::tests::run_shell;
    use crate::process::Termination;

    #[test]
    fn captures_stdout() {
        let (term, output) = run_shell("echo hello");
        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("hello"), "{text:?}");
    }

    #[test]
    fn merges_stderr_into_the_same_stream() {
        let (term, output) = run_shell("echo out& echo err 1>&2& echo out2");
        assert_eq!(term, Termination::Success);
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("out"), "{text:?}");
        assert!(text.contains("err"), "{text:?}");
        assert!(text.contains("out2"), "{text:?}");
    }
}
