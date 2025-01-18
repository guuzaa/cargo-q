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
        // Skip the first argument which is "q" for cargo subcommands
        let mut args = std::env::args().collect::<Vec<_>>();
        if args.len() >= 2 && args[1] == "q" {
            args.remove(1);
        }

        Self::parse_from(args)
    }
}
