use crate::process::Termination;
use crate::progress;
use crate::routine::Routine;
use crate::strategy::{Parallel, Repeat, Sequential, Strategy};
use std::io::{self, Write};

/// How a list of routines should be run.
///
/// Grouped into one value so that adding a knob does not add another `bool`
/// to every layer between the CLI and a strategy.
#[derive(Debug, Clone, Copy)]
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
    /// Allow the overprinting "fancy" console, and the colored captured
    /// output that goes with it, on a terminal.
    pub fancy: bool,
    /// Run the whole list of commands this many times.
    ///
    /// One means a single pass, which is what every run does without
    /// `--repeat`. The rounds run one after another, so this cannot be
    /// combined with `parallel`: the executor rejects that, and so does the
    /// command line. The count is a `u32`, which is more rounds than any
    /// run sits through.
    pub repeat: u32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            verbose: false,
            parallel: false,
            keep_going: false,
            dry_run: false,
            fancy: true,
            repeat: 1,
        }
    }
}

impl Options {
    /// Parallel commands cannot each inherit the TTY, so `--verbose` is
    /// dropped and the run stays quiet.
    fn supported(self) -> Self {
        if self.verbose && self.parallel {
            eprintln!(
                "warning: -v/--verbose is not supported with -p/--parallel; running quietly\n\
                 help: use -v/--verbose only to watch each command's output"
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

        // The rounds of a repeated run go one after another, so there is
        // nothing for `--parallel` to do. The command line rejects the
        // combination before it gets here; the library API rejects it here.
        if options.repeat > 1 && options.parallel {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a repeated run cannot be parallel: the rounds run one after another",
            ));
        }

        if options.dry_run {
            self.print_plan()?;
            return Ok(Termination::Success);
        }

        // The reporter prints the run's summary when dropped, so it is owned
        // here: it outlives the strategy and is gone before the outcome
        // reaches `main`, which may print an error of its own.
        //
        // The reporter counts every command the run starts, but it only
        // needs a capture slot per command of the list: the rounds of a
        // repeated run go one after another and reuse the slots.
        let progress = progress::new(self.total(options), self.routines.len(), options);
        if options.repeat > 1 {
            Repeat.execute(&self.routines, &progress, options)
        } else if options.parallel {
            Parallel.execute(&self.routines, &progress, options)
        } else {
            Sequential.execute(&self.routines, &progress, options)
        }
    }

    /// How many commands the run starts in total.
    ///
    /// Every round runs the whole list, so a repeated run starts each
    /// command once per round.
    fn total(&self, options: Options) -> usize {
        // The count is a `u32`, which fits in a `usize` on every target
        // Rust supports.
        self.routines
            .len()
            .saturating_mul(options.repeat.max(1) as usize)
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

        // The rounds are named only when there are more than one: "2
        // command(s), sequentially:" reads better than "2 command(s) x 1
        // rounds, sequentially:".
        let rounds = match self.options.repeat.max(1) {
            1 => String::new(),
            rounds => format!(" x {rounds} rounds"),
        };

        let stdout = io::stdout();
        let mut out = stdout.lock();
        writeln!(out, "{} command(s){rounds}, {mode}:", self.routines.len())?;
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

    #[test]
    fn a_single_round_is_the_default() {
        assert_eq!(Options::default().repeat, 1);
    }

    /// The rounds go one after another, so there is nothing for the
    /// parallel strategy to do. The command line rejects the combination;
    /// the library API rejects it too.
    #[test]
    fn a_repeated_run_cannot_be_parallel() {
        let options = Options {
            repeat: 2,
            parallel: true,
            ..Default::default()
        };
        let err = Executor::new(Vec::new(), options).execute().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{err}");
    }
}
