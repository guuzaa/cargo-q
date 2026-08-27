//! Command execution progress reporting.
//!
//! Mirrors n2's progress split: a "fancy" console overprints a live status
//! when both stdin and stdout are terminals; otherwise a "dumb" console
//! prints one line per command.

mod color;
mod dumb;
mod fancy;

pub(crate) use color::ColorExt;
use dumb::DumbConsoleProgress;
use fancy::{use_fancy, FancyConsoleProgress};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Instant;

/// Trait for command progress notifications.
pub trait Progress: Send + Sync {
    /// Called when a command starts.
    fn task_started(&self, id: usize, cmd: &str);

    /// Called with a chunk of merged stdout+stderr as the command produces it.
    fn task_output(&self, id: usize, data: &[u8]);

    /// Called when a command completes.
    fn task_finished(&self, id: usize, cmd: &str, success: bool);
}

/// Build a progress reporter for `total` commands.
///
/// Fancy overprinting is only used on an interactive terminal and when
/// commands do not inherit stdio (`verbose` is false); otherwise cargo
/// output would collide with the status display.
pub fn new_progress(total: usize, verbose: bool) -> Arc<dyn Progress> {
    if use_fancy() && !verbose {
        Arc::new(FancyConsoleProgress::new(total, verbose))
    } else {
        Arc::new(DumbConsoleProgress::new(total, verbose))
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
    writeln!(handle, "{} {} command(s) in {:.2}s", status, total, elapsed)
        .expect("write stdio failed");
    handle.flush().expect("flush stdio failed")
}

#[cfg(test)]
mod tests {
    use super::append_stream;

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
