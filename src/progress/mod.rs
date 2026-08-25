//! Command execution progress reporting.
//!
//! Mirrors n2's progress split: a "fancy" console overprints a live status
//! when both stdin and stdout are terminals; otherwise a "dumb" console
//! prints one line per command.

mod dumb;
mod fancy;

use dumb::DumbConsoleProgress;
use fancy::FancyConsoleProgress;
use std::fmt;
use std::io::{self, IsTerminal};
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

/// Fancy progress is used when both stdin and stdout are terminals.
pub fn use_fancy() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
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

pub(crate) trait ColorExt {
    fn red(self) -> ColoredString;
    fn green(self) -> ColoredString;
    fn bold(self) -> ColoredString;
}

pub(crate) struct ColoredString(String);

impl fmt::Display for ColoredString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn color_enabled() -> bool {
    io::stdout().is_terminal()
}

impl<T: fmt::Display> ColorExt for T {
    fn red(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[31m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn green(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[32m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
    fn bold(self) -> ColoredString {
        if color_enabled() {
            ColoredString(format!("\x1b[1m{}\x1b[0m", self))
        } else {
            ColoredString(self.to_string())
        }
    }
}

pub(crate) fn print_summary(success: usize, started: usize, total: usize, start_time: Instant) {
    if success != total {
        println!(
            "\n{} succeeded, {} failed, {} skipped",
            success,
            started.saturating_sub(success),
            total.saturating_sub(started)
        );
    }
    let elapsed = start_time.elapsed().as_secs_f32();
    let status = if success == total {
        "Finished".green()
    } else {
        "Failed".red()
    };
    println!("{} {} command(s) in {:.2}s\n", status, total, elapsed);
}
