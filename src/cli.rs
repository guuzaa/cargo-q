use crate::executor::Executor;
use crate::routine::Routine;
use clap::Parser;
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
    /// Commands are separated by spaces:
    ///
    ///   e.g., check test run
    ///
    /// Note: For commands with arguments, you need to quote the entire command:
    ///
    ///   e.g., "test --features f1" "run --release"
    #[arg(required = true, allow_hyphen_values = true)]
    pub commands: Vec<Routine>,

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
    pub fn parse() -> Self {
        // Skip the all arguments which are "q" for cargo subcommands
        let args = std::env::args()
            .filter(|arg| arg != "q")
            .collect::<Vec<_>>();

        Self::parse_from(args)
    }

    pub fn run(self) -> io::Result<()> {
        Executor::new(self.commands, self.parallel, self.verbose).execute()
    }
}
