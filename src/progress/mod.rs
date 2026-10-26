//! Command execution progress reporting.
//!
//! Mirrors n2's progress split: a "fancy" console overprints a live status
//! when stdout is a terminal; otherwise a "dumb" console prints one line per
//! command. Sequential `--verbose` always uses the dumb console so cargo can
//! own the TTY.

mod color;
mod dumb;
mod fancy;
mod output;

use crate::executor::Options;
pub(crate) use color::Colored;
pub(crate) use output::Tail;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

/// Trait for command progress notifications.
pub trait Progress: Send + Sync {
    /// Called when a command starts.
    ///
    /// `id` is the command's place in the list, and it tells the commands
    /// that run at the same time apart. A repeated run reuses the same id
    /// for the same command in every round: the rounds run one after
    /// another, so only one round's commands are live at a time.
    fn task_started(&self, id: usize, cmd: &str);

    /// Called with a chunk of merged stdout+stderr as the command produces it.
    fn task_output(&self, id: usize, data: &[u8]);

    /// Called when a command completes.
    ///
    /// `code` is the child's exit status when it exited non-zero on its own.
    /// It is `None` when there is no status to report: the command could not
    /// be spawned, or cargo-q itself was interrupted.
    fn task_finished(&self, id: usize, cmd: &str, success: bool, code: Option<i32>);
}

/// Build a progress reporter for `total` commands.
///
/// `total` is how many commands the run starts in all: a repeated run
/// starts every command once per round. `slots` is how many of them can run
/// at the same time, and how many buffers are set aside to capture their
/// output. A repeated run runs its rounds one after another and reuses the
/// slots, so `slots` is the length of the command list, not `total`.
///
/// Fancy overprinting is used on an interactive terminal, except when
/// `--verbose`: those commands inherit the TTY so cargo can draw its own
/// progress, and a status overlay would collide with it. Parallel
/// `--verbose` is rejected before this is called, so `verbose` here is
/// sequential only. `--no-fancy` rules the fancy console out altogether.
#[must_use]
pub fn new(total: usize, slots: usize, options: Options) -> Arc<dyn Progress> {
    let tty = fancy::enabled();
    // Color belongs to a person reading a terminal; `--no-fancy` opts out of
    // it together with the overprinting display.
    color::set_enabled(tty && options.fancy);
    if want_fancy(tty, options.verbose, options.fancy) {
        Arc::new(fancy::ConsoleProgress::new(total, slots))
    } else {
        Arc::new(dumb::ConsoleProgress::new(total, slots, options.verbose))
    }
}

/// `--verbose` inherits the TTY; overlaying a status bar on cargo's own
/// progress would collide.
fn want_fancy(tty: bool, verbose: bool, fancy: bool) -> bool {
    tty && !verbose && fancy
}

/// The line that introduces a failed command's captured output.
///
/// The exit code belongs in the line, not only in the process status: with
/// `--keep-going` several commands can fail, and only the first one's code
/// reaches the shell.
pub(crate) fn failure_line(cmd: &str, code: Option<i32>) -> String {
    match code {
        Some(code) => format!("failed: {cmd} (exit {code})"),
        None => format!("failed: {cmd}"),
    }
}

/// Append captured streams to `buf`.
/// matching how they appear when a command inherits the terminal.
/// Each non-empty stream is given a trailing newline.
pub(crate) fn append_stream(buf: &mut Vec<u8>, data: &[u8]) {
    if data.is_empty() {
        return;
    }
    buf.extend_from_slice(data);
    if !data.ends_with(b"\n") {
        buf.push(b'\n');
    }
}

pub(crate) fn truncate(s: &str, mut max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    while !s.is_char_boundary(max) {
        max -= 1;
    }
    &s[..max]
}

pub(crate) fn print_summary(success: usize, started: usize, total: usize, start_time: Instant) {
    let elapsed = start_time.elapsed().as_secs_f32();
    let status = if crate::process::was_interrupted() {
        "Interrupted".yellow()
    } else if success == total {
        "Finished".green()
    } else {
        "Failed".red()
    };

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if success != total {
        writeln!(
            handle,
            "\n{} succeeded, {} failed, {} skipped",
            success,
            started.saturating_sub(success),
            total.saturating_sub(started)
        )
        .expect("write stdio failed");
    }
    writeln!(handle, "{status} {total} command(s) in {elapsed:.2}s").expect("write stdio failed");
    handle.flush().expect("flush stdio failed");
}

#[cfg(test)]
mod tests {
    use super::{append_stream, failure_line};

    #[test]
    fn sequential_verbose_skips_fancy_even_on_a_tty() {
        assert!(
            !super::want_fancy(true, true, true),
            "sequential -v: cargo owns the TTY"
        );
        assert!(
            super::want_fancy(true, false, true),
            "quiet still overlays on a TTY"
        );
        assert!(!super::want_fancy(false, false, true));
        assert!(!super::want_fancy(true, false, false));
        assert!(!super::want_fancy(true, true, false));
    }

    /// The code is what makes several failures in one `--keep-going` run
    /// distinguishable, since only the first reaches the shell.
    #[test]
    fn failure_line_carries_the_exit_code() {
        assert_eq!(
            failure_line("cargo clippy", Some(101)),
            "failed: cargo clippy (exit 101)"
        );
        assert_eq!(failure_line("cargo clippy", None), "failed: cargo clippy");
    }

    #[test]
    fn captured_output_stdout_then_stderr() {
        let mut buf = Vec::new();
        append_stream(&mut buf, b"test foo ... FAILED\n");
        append_stream(&mut buf, b"error: test failed\n");
        assert_eq!(buf, b"test foo ... FAILED\nerror: test failed\n");
    }

    #[test]
    fn captured_output_adds_missing_newlines() {
        let mut buf = Vec::new();
        append_stream(&mut buf, b"out");
        append_stream(&mut buf, b"err");
        assert_eq!(buf, b"out\nerr\n");
    }

    #[test]
    fn captured_output_skips_empty_streams() {
        let mut buf = Vec::new();
        append_stream(&mut buf, b"");
        append_stream(&mut buf, b"err\n");
        assert_eq!(buf, b"err\n");

        buf.clear();
        append_stream(&mut buf, b"out\n");
        append_stream(&mut buf, b"");
        assert_eq!(buf, b"out\n");
    }
}
