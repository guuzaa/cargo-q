use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cargo-q")]
#[command(version)]
#[command(about = "A cargo subcommand for running multiple cargo commands in a time")]
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
    pub commands: Vec<String>,

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
}
