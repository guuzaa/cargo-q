use cargo_q::cli::Cli;
use std::io::ErrorKind;

fn main() {
    match Cli::parse().run() {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::Interrupted => {
            std::process::exit(130);
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
