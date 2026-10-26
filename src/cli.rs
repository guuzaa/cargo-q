use crate::executor::{Executor, Options};
use crate::process::{self, Termination};
use crate::routine::Routine;
use clap::Parser;
use std::ffi::OsString;
use std::io;

#[derive(Parser, Debug)]
#[command(name = "cargo-q")]
#[command(version)]
#[command(about = "Run Cargo only once.")]
#[command(author)]
pub struct Cli {
    /// Commands to execute
    ///
    /// A token that does not start with `-` starts a new command; `-` tokens
    /// are arguments to the preceding one:
    ///
    ///   check test, build -r test --no-run
    ///
    /// An argument that does not start with `-` needs quoting or `=`, even
    /// when it collides with a subcommand name (e.g. a feature called
    /// `test`):
    ///
    ///   "test --features f1", build --features=f1, "build --features test"
    #[arg(required = true, allow_hyphen_values = true, trailing_var_arg = true)]
    commands: Vec<OsString>,

    /// Show each command's output as it runs
    ///
    /// Sequential only. Combined with `--parallel`, cargo-q warns and
    /// ignores `--verbose` so the run stays quiet.
    #[arg(short, long)]
    pub verbose: bool,

    /// Run all commands at once instead of one after another (experimental)
    ///
    /// Commands that share the target directory lock each other out, so
    /// `check`/`build`/`test` may not finish any sooner than in sequence.
    /// `--verbose` is ignored in this mode; use sequential `-v` instead.
    #[arg(short, long)]
    pub parallel: bool,

    /// Keep running the remaining commands after one fails
    ///
    /// By default cargo-q stops at the first failure, like `&&` in a shell.
    /// In parallel mode only queued commands are skipped; one already running
    /// is left to finish.
    ///
    /// With `--repeat`, the later rounds run as well.
    #[arg(short, long)]
    pub keep_going: bool,

    /// Print the commands that would run, without running them
    ///
    /// Shows how a command line was split, since a bare token can be read as
    /// a new command rather than an argument.
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Print plain output, without the overprinting status display
    ///
    /// Keeps one line per command and no ANSI color even on a terminal, for
    /// output that is captured rather than read by a person: an agent driving
    /// the build through a pseudo-terminal, a log file, a CI console.
    #[arg(long)]
    pub no_fancy: bool,

    /// Run the whole list of commands this many times
    ///
    /// A round is the whole list, in the order the commands were given, so
    /// `--repeat 3 check test` runs `check test` three times over.
    ///
    /// Without it the list runs once.
    ///
    /// A failure ends the run, so the later rounds never start. With
    /// `--keep-going` every round runs anyway, and every command runs once
    /// per round even when an earlier one failed.
    ///
    /// The rounds run one after another, so this cannot be combined with
    /// `--parallel`.
    #[arg(long, value_name = "N", value_parser = repeat_count,
          conflicts_with = "parallel")]
    pub repeat: Option<usize>,
}

/// The number of rounds to run.
///
/// Zero rounds would run nothing at all, so it is rejected rather than
/// quietly turning into a successful no-op.
fn repeat_count(value: &str) -> Result<usize, String> {
    let count: usize = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number of rounds"))?;
    if count == 0 {
        return Err("a run needs at least one round".to_string());
    }
    Ok(count)
}

impl Cli {
    #[must_use]
    pub fn parse() -> Self {
        // Skip the all arguments which are "q" for cargo subcommands
        let args = std::env::args_os()
            .filter(|arg| arg != "q")
            .collect::<Vec<_>>();

        Self::parse_from(args)
    }

    /// Parse-and-run entry point.
    ///
    /// The interrupt handler is installed before parsing, because resolving
    /// subcommand names spawns `cargo --list` too, and a signal during that
    /// window must end in a clean `Interrupted` instead of the default action.
    pub fn run(self) -> io::Result<Termination> {
        process::install_interrupt_handler();

        if process::was_interrupted() {
            return Ok(Termination::Interrupted);
        }

        let routines = match Routine::parse_many(&self.commands) {
            Ok(routines) => routines,
            // A signal that arrives while names are being resolved must not
            // surface as a misleading parse error.
            Err(_) if process::was_interrupted() => return Ok(Termination::Interrupted),
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
        };
        let options = Options {
            verbose: self.verbose,
            parallel: self.parallel,
            keep_going: self.keep_going,
            dry_run: self.dry_run,
            fancy: !self.no_fancy,
            repeat: self.repeat.unwrap_or(1),
        };
        Executor::new(routines, options).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphen_values_are_captured() {
        let cli = Cli::parse_from(["cargo-q", "build", "-r", "test", "--no-run"]);
        assert_eq!(
            cli.commands,
            ["build", "-r", "test", "--no-run"].map(OsString::from)
        );
        assert!(!cli.parallel);
        assert!(!cli.verbose);

        let routines = Routine::parse_many(&cli.commands).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].to_string(), "cargo build -r");
        assert_eq!(routines[1].to_string(), "cargo test --no-run");
    }

    #[test]
    fn flags_before_commands_still_work() {
        let cli = Cli::parse_from(["cargo-q", "-p", "-v", "build", "-r"]);
        assert!(cli.parallel);
        assert!(cli.verbose);
        assert_eq!(cli.commands, ["build", "-r"].map(OsString::from));
    }

    #[test]
    fn run_control_flags_default_to_off() {
        let cli = Cli::parse_from(["cargo-q", "check"]);
        assert!(!cli.keep_going, "cargo-q stops at the first failure");
        assert!(!cli.dry_run);
    }

    #[test]
    fn run_control_flags_are_accepted() {
        let cli = Cli::parse_from(["cargo-q", "-k", "-n", "check", "test"]);
        assert!(cli.keep_going);
        assert!(cli.dry_run);
        assert_eq!(cli.commands, ["check", "test"].map(OsString::from));

        let cli = Cli::parse_from(["cargo-q", "--keep-going", "--dry-run", "check"]);
        assert!(cli.keep_going);
        assert!(cli.dry_run);
    }

    /// A flag only belongs to cargo-q before the first command; after it, the
    /// parser hands every `-` token to the preceding cargo command.
    #[test]
    fn a_flag_after_a_command_is_not_a_cargo_q_flag() {
        let cli = Cli::parse_from(["cargo-q", "check", "--dry-run"]);
        assert!(!cli.dry_run);
        assert_eq!(cli.commands, ["check", "--dry-run"].map(OsString::from));
    }

    #[test]
    fn no_fancy_is_accepted() {
        let cli = Cli::parse_from(["cargo-q", "--no-fancy", "check"]);
        assert!(cli.no_fancy);

        let cli = Cli::parse_from(["cargo-q", "check"]);
        assert!(!cli.no_fancy, "the overprinting console is the default");
    }

    #[test]
    fn repeat_is_absent_by_default() {
        let cli = Cli::parse_from(["cargo-q", "check"]);
        assert_eq!(cli.repeat, None);
    }

    #[test]
    fn repeat_is_accepted_before_the_commands() {
        let cli = Cli::parse_from(["cargo-q", "--repeat", "3", "check", "test"]);
        assert_eq!(cli.repeat, Some(3));
        assert_eq!(cli.commands, ["check", "test"].map(OsString::from));

        let cli = Cli::parse_from(["cargo-q", "--repeat=3", "check"]);
        assert_eq!(cli.repeat, Some(3));
    }

    /// Zero rounds would run nothing at all, which is not what asking for
    /// a repeat means.
    #[test]
    fn repeat_rejects_zero_rounds() {
        let err = Cli::try_parse_from(["cargo-q", "--repeat", "0", "check"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
    }

    /// The rounds run one after another, so there is nothing for
    /// `--parallel` to do.
    #[test]
    fn repeat_conflicts_with_parallel() {
        let err = Cli::try_parse_from(["cargo-q", "--repeat", "3", "-p", "check"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);

        // The default is not a conflict: `--parallel` alone still works.
        let cli = Cli::parse_from(["cargo-q", "-p", "check"]);
        assert!(cli.parallel);
        assert_eq!(cli.repeat, None);
    }

    #[test]
    fn a_repeat_after_a_command_is_not_a_cargo_q_flag() {
        let cli = Cli::parse_from(["cargo-q", "check", "--repeat", "3"]);
        assert_eq!(cli.repeat, None);
        assert_eq!(cli.commands, ["check", "--repeat", "3"].map(OsString::from));
    }
}
