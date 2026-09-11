use crate::executor::Executor;
use crate::process::{self, Termination};
use crate::routine::Routine;
use clap::Parser;
use std::ffi::OsString;
use std::io;

#[derive(Parser, Debug)]
#[command(name = "cargo-q")]
#[command(version)]
#[command(
    about = "A Cargo subcommand for running multiple Cargo commands sequentially or in parallel."
)]
#[command(author)]
pub struct Cli {
    /// Commands to execute
    ///
    /// A token that does not start with `-` starts a new command; following
    /// tokens that start with `-` are its arguments:
    ///
    ///   e.g., check test, build -r test --no-run
    ///
    /// Quote a command when an argument does not
    /// start with `-`, including when that argument is itself a subcommand
    /// name (e.g. a feature called `test`):
    ///
    ///   e.g., "test --features f1" , "build --features test"
    ///
    /// If you'd rather not quote, attach the value with `=`:
    ///
    ///   e.g., build --features=test
    #[arg(required = true, allow_hyphen_values = true, trailing_var_arg = true)]
    commands: Vec<OsString>,

    /// Run commands in verbose mode
    ///
    /// Shows the output of each command as it runs
    #[arg(short, long)]
    pub verbose: bool,

    /// Run commands in parallel
    ///
    /// Runs all commands in parallel instead of sequentially
    #[arg(short, long)]
    pub parallel: bool,
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
        Executor::new(routines, self.parallel, self.verbose).execute()
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
}
