use std::fmt;
use std::time::Instant;

// Terminal colors support
pub trait ColorExt {
    fn red(self) -> ColoredString;
    fn green(self) -> ColoredString;
    fn bold(self) -> ColoredString;
}

impl<T: fmt::Display> ColorExt for T {
    fn red(self) -> ColoredString {
        ColoredString(format!("\x1b[31m{}\x1b[0m", self))
    }
    fn green(self) -> ColoredString {
        ColoredString(format!("\x1b[32m{}\x1b[0m", self))
    }
    fn bold(self) -> ColoredString {
        ColoredString(format!("\x1b[1m{}\x1b[0m", self))
    }
}

pub struct ColoredString(String);

impl fmt::Display for ColoredString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct ExecutionSummary {
    success_count: usize,
    total_commands: usize,
    start_time: Instant,
}

impl ExecutionSummary {
    pub fn new(total_commands: usize) -> Self {
        Self {
            success_count: 0,
            total_commands,
            start_time: Instant::now(),
        }
    }

    pub fn increment_success(&mut self) {
        self.success_count += 1;
    }

    fn print_summary(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let status = if self.success_count == self.total_commands {
            "Finished".green()
        } else {
            "Failed".red()
        };
        println!(
            "\n{} {} command(s) in {:.2}s",
            status, self.total_commands, elapsed
        );
        if self.success_count != self.total_commands {
            println!(
                "  {} succeeded, {} failed",
                self.success_count,
                self.total_commands - self.success_count
            );
        }
    }
}

impl Drop for ExecutionSummary {
    fn drop(&mut self) {
        self.print_summary();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_summary() {
        let mut summary = ExecutionSummary::new(3);
        assert_eq!(summary.success_count, 0);
        assert_eq!(summary.total_commands, 3);

        summary.increment_success();
        assert_eq!(summary.success_count, 1);

        summary.increment_success();
        assert_eq!(summary.success_count, 2);

        summary.increment_success();
        assert_eq!(summary.success_count, 3);
    }
}
