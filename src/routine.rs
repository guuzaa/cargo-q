use std::ffi::OsString;
use std::fmt;
use std::io;
use std::process::{Command, Output, Stdio};
use std::str::FromStr;

#[derive(Debug, Default, Clone)]
pub struct Routine {
    name: String,
    args: Vec<String>,
}

impl fmt::Display for Routine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cargo {}", self.name)?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

impl Routine {
    pub fn run(&self, verbose: bool) -> io::Result<(bool, Output)> {
        let mut cmd = Command::new(cargo_bin());
        cmd.arg(&self.name).args(&self.args);

        if verbose {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
            let status = cmd.status()?;
            Ok((
                status.success(),
                Output {
                    status,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            ))
        } else {
            let output = cmd.output()?;
            Ok((output.status.success(), output))
        }
    }
}

/// Locate the cargo binary.
///
/// Cargo sets `CARGO` to the path of the cargo binary running this
/// subcommand, which both avoids a PATH lookup and guarantees the same
/// toolchain is used for the spawned commands. Fall back to `cargo` on
/// PATH when invoked without cargo (e.g. running the binary directly).
#[inline]
fn cargo_bin() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

impl FromStr for Routine {
    type Err = String;

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return Err("command must not be empty".to_string());
        }

        Ok(Routine {
            name: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        for cmd in ["check", "test", "run"] {
            let routine = Routine::from_str(cmd).unwrap();
            assert_eq!(routine.name, cmd);
            assert!(routine.args.is_empty());
        }
    }

    #[test]
    fn test_parse_with_args() {
        let routine = Routine::from_str("test --features feature1").unwrap();
        assert_eq!(routine.name, "test");
        assert_eq!(routine.args, vec!["--features", "feature1"]);

        let routine = Routine::from_str("run --release").unwrap();
        assert_eq!(routine.name, "run");
        assert_eq!(routine.args, vec!["--release"]);
    }

    #[test]
    fn test_parse_empty() {
        assert!(Routine::from_str("").is_err());
        assert!(Routine::from_str("   ").is_err());
    }
}
