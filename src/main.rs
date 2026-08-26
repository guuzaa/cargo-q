mod cli;
mod executor;
mod progress;
mod routine;
mod strategy;
mod thread_pool;

use cli::Cli;

fn main() {
    if let Err(e) = Cli::parse().run() {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
