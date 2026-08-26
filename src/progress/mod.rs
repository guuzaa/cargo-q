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

    /// Called when a command completes.
    fn task_finished(&self, id: usize, cmd: &str, success: bool, stderr: &[u8]);

    /// Log a line of output without corrupting the progress display.
    fn log(&self, msg: &str);
}

/// Build a progress reporter for `total` commands.
///
/// Fancy overprinting is only used on an interactive terminal and when
/// commands do not inherit stdio (`verbose` is false); otherwise cargo
/// output would collide with the status display.
pub fn new_progress(total: usize, verbose: bool) -> Arc<dyn Progress> {
    if use_fancy() && !verbose {
        Arc::new(FancyConsoleProgress::new(total))
    } else {
        Arc::new(DumbConsoleProgress::new(total))
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
    let status = if success == total {
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
