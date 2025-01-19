use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "cargo-q")]
pub struct Cli {
    /// Commands to execute
    pub command_string: String,

    /// Run in verbose mode
    #[arg(short, long)]
    pub verbose: bool,

    /// Run commands in parallel
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
