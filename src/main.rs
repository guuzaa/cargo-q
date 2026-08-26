mod cli;
mod executor;
mod progress;
mod routine;
mod strategy;
mod thread_pool;

use cli::Cli;
use executor::Executor;

fn main() {
    let cli = Cli::parse();
    let executor = Executor::new(&cli.commands, cli.parallel, cli.verbose);
    if let Err(e) = executor.execute() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
