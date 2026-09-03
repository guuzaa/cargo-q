use crate::process::{self, Termination};
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::str::FromStr;

#[derive(Debug, Default, Clone)]
pub struct Routine {
    bin: OsString,
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
    /// Run this routine. The executable was resolved once, when the
    /// routine was parsed, so execution strategies never need to know
    /// anything about which binary is being invoked.
    pub fn run(&self, verbose: bool, output_cb: impl FnMut(&[u8])) -> io::Result<Termination> {
        let mut args = Vec::with_capacity(1 + self.args.len());
        args.push(self.name.as_str());
        args.extend(self.args.iter().map(String::as_str));
        process::run_command(&self.bin, args, verbose, output_cb)
    }
}

impl FromStr for Routine {
    type Err = String;

    fn from_str(cmd: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return Err("command must not be empty".to_string());
        }

        Ok(Routine {
            bin: process::cargo_bin(),
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
