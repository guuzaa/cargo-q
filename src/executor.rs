use crate::process::Termination;
use crate::progress;
use crate::routine::Routine;
use crate::strategy::{Parallel, Sequential, Strategy};
use std::io::{self, Write};

/// How a list of routines should be run.
///
/// Grouped into one value so that adding a knob does not add another `bool`
/// to every layer between the CLI and a strategy.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Let commands inherit this process's stdio instead of capturing it.
    ///
    /// Ignored when `parallel` is set: the TTY cannot be shared, so the run
    /// stays quiet.
    pub verbose: bool,
    /// Run all commands at once instead of one after another.
    pub parallel: bool,
    /// Keep running the remaining commands after one fails.
    pub keep_going: bool,
    /// Print the commands that would run, and run nothing.
    pub dry_run: bool,
}

impl Options {
    /// Parallel commands cannot each inherit the TTY, so `--verbose` is
    /// dropped and the run stays quiet.
    fn supported(self) -> Self {
        if self.verbose && self.parallel {
            eprintln!(
                "warning: --verbose is not supported with --parallel; running quietly\n\
                 help: use sequential -v to watch each command's output"
            );
            Self {
                verbose: false,
                ..self
            }
        } else {
            self
        }
    }
}

pub(crate) struct Executor {
    routines: Vec<Routine>,
    options: Options,
}

impl Executor {
    pub fn new(routines: Vec<Routine>, options: Options) -> Self {
        Executor { routines, options }
    }

    /// Run the routines with the selected strategy.
    pub fn execute(&self) -> io::Result<Termination> {
        let options = self.options.supported();
        if options.dry_run {
            self.print_plan()?;
            return Ok(Termination::Success);
        }

        // The reporter prints the run's summary when dropped, so it is owned
        // here: it outlives the strategy and is gone before the outcome
        // reaches `main`, which may print an error of its own.
        let progress = progress::new(self.routines.len(), options.verbose);
        if options.parallel {
            Parallel.execute(&self.routines, &progress, options)
        } else {
            Sequential.execute(&self.routines, &progress, options)
        }
    }

    /// Print the commands that would run, one per line.
    ///
    /// Turning a command line into routines involves guesswork cargo-q cannot
    /// always get right — a bare token may name a subcommand or be an argument
    /// to the previous one — so this is the way to see what an invocation
    /// means before it spawns anything. The header is a separate line, so the
    /// commands can be read off with `tail -n +2`.
    fn print_plan(&self) -> io::Result<()> {
        let mode = if self.options.parallel {
            "in parallel"
        } else {
            "sequentially"
        };

        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{} command(s), {mode}:", self.routines.len())?;
        for routine in &self.routines {
            writeln!(out, "  {routine}")?;
        }
        out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_verbose_runs_quietly() {
        let options = Options {
            verbose: true,
            parallel: true,
            ..Default::default()
        }
        .supported();
        assert!(!options.verbose, "parallel -v is ignored");
        assert!(options.parallel);
    }

    #[test]
    fn sequential_verbose_is_kept() {
        let options = Options {
            verbose: true,
            ..Default::default()
        }
        .supported();
        assert!(options.verbose);
        assert!(!options.parallel);
    }

    #[test]
    fn parallel_quiet_is_unchanged() {
        let options = Options {
            parallel: true,
            ..Default::default()
        }
        .supported();
        assert!(!options.verbose);
        assert!(options.parallel);
    }
}
