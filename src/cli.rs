use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cargo-q")]
#[command(version)]
#[command(about = "A cargo subcommand for running multiple cargo commands in a time")]
#[command(author)]
pub struct Cli {
    /// Commands to execute
    /// 
    /// Supports multiple separators:
    /// 
    ///   space: Independent commands (e.g., "check test")
    /// 
    ///   ;    : Independent commands with args (e.g., "test --features f1 ; run")
    /// 
    ///   &    : Dependent commands (e.g., "check & test & run")
    pub command_string: String,

    /// Run commands in verbose mode
    /// 
    /// Shows the output of each command as it runs
    #[arg(short, long)]
    pub verbose: bool,

    /// Run commands in parallel
    /// 
    /// Only works with independent commands (space or ; separator)
    #[arg(short, long)]
    pub parallel: bool,
}

impl Cli {
    pub fn parse() -> Self {
        // Skip the all arguments which are "q" for cargo subcommands
        let args = std::env::args()
            .collect::<Vec<_>>()
            .into_iter()
            .filter(|arg| arg != "q")
            .collect::<Vec<_>>();

        Self::parse_from(args)
    }
}
