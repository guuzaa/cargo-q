mod cli;
mod executor;
mod parser;
mod process;
mod routine;
mod strategy;
mod thread_pool;

use cli::Cli;
use parser::Parser;

fn main() {
    let cli = Cli::parse();
    let executor = Parser.parse(&cli.commands, cli.parallel, cli.verbose);
    if let Err(e) = executor.execute() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
