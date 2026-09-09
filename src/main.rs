use cargo_q::cli::Cli;
use cargo_q::process::Termination;
use std::io::ErrorKind;
use std::process::ExitCode;

fn main() -> ExitCode {
    match Cli::parse().run() {
        Ok(term) => exit_code(term),
        Err(e) if e.kind() == ErrorKind::Interrupted => ExitCode::from(130),
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}

/// Map an outcome to a process exit code.
///
/// `ExitCode` carries a `u8`, while a child's status is a full `i32` on
/// Windows. Unix statuses always fit; anything wider degrades to a plain
/// failure rather than wrapping into a bogus small code.
fn exit_code(term: Termination) -> ExitCode {
    u8::try_from(term.exit_code())
        .map(ExitCode::from)
        .unwrap_or(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_outcomes_to_exit_codes() {
        assert_eq!(exit_code(Termination::Success), ExitCode::SUCCESS);
        assert_eq!(exit_code(Termination::Interrupted), ExitCode::from(130));
        assert_eq!(exit_code(Termination::Failure(101)), ExitCode::from(101));
        // Out of `u8` range, e.g. a Windows status code: a plain failure.
        assert_eq!(exit_code(Termination::Failure(1000)), ExitCode::FAILURE);
    }
}
