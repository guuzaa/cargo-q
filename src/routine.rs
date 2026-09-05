use crate::process::{self, Termination};
use std::ffi::OsString;
use std::fmt;
use std::io;

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
    fn new(name: String, args: Vec<String>) -> Self {
        Self {
            bin: process::cargo_bin(),
            name,
            args,
        }
    }

    /// Parse a single command. The first word is the cargo subcommand; every
    /// following word is an argument, whether or not it starts with `-`.
    fn parse_one(cmd: &str) -> Result<Self, String> {
        let mut parts = cmd.split_whitespace();
        let name = parts
            .next()
            .ok_or_else(|| "command must not be empty".to_string())?;
        Ok(Self::new(
            name.to_string(),
            parts.map(str::to_string).collect(),
        ))
    }

    /// Parse one or more routines from command-line tokens.
    ///
    /// A token that does not start with `-` starts a new command. Subsequent
    /// tokens that start with `-` are arguments to that command.
    ///
    /// A token that contains whitespace is a complete command (name plus
    /// arguments), matching quoted CLI arguments such as `"test --features f1"`.
    pub fn parse<I, S>(tokens: I) -> Result<Vec<Self>, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut routines = Vec::new();

        for token in tokens {
            let token = token.as_ref();
            if token.split_whitespace().nth(1).is_some() {
                routines.push(Self::parse_one(token)?);
                continue;
            }

            let token = token.trim();
            if token.is_empty() {
                return Err("command must not be empty".to_string());
            }

            if token.starts_with('-') {
                match routines.last_mut() {
                    Some(routine) => routine.args.push(token.to_string()),
                    None => {
                        return Err(format!(
                            "unexpected argument '{token}': a command must come first"
                        ));
                    }
                }
            } else {
                routines.push(Self::new(token.to_string(), Vec::new()));
            }
        }

        if routines.is_empty() {
            return Err("command must not be empty".to_string());
        }

        Ok(routines)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_many_bare_commands() {
        let routines = Routine::parse(["check", "test", "run"]).unwrap();
        assert_eq!(routines.len(), 3);
        assert_eq!(routines[0].name, "check");
        assert!(routines[0].args.is_empty());
        assert_eq!(routines[1].name, "test");
        assert!(routines[1].args.is_empty());
        assert_eq!(routines[2].name, "run");
        assert!(routines[2].args.is_empty());
    }

    #[test]
    fn test_parse_many_flag_args() {
        let routines = Routine::parse(["build", "-r", "test", "--no-run"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "build");
        assert_eq!(routines[0].args, vec!["-r"]);
        assert_eq!(routines[1].name, "test");
        assert_eq!(routines[1].args, vec!["--no-run"]);
    }

    #[test]
    fn test_parse_many_multiple_flags() {
        let routines = Routine::parse(["build", "-r", "--offline", "test"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "build");
        assert_eq!(routines[0].args, vec!["-r", "--offline"]);
        assert_eq!(routines[1].name, "test");
        assert!(routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_many_quoted_keeps_non_flag_args() {
        let routines = Routine::parse(["test --features feature1", "build", "-r"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "test");
        assert_eq!(routines[0].args, vec!["--features", "feature1"]);
        assert_eq!(routines[1].name, "build");
        assert_eq!(routines[1].args, vec!["-r"]);
    }

    #[test]
    fn test_parse_many_leading_flag_is_error() {
        let err = Routine::parse(["-r", "test"]).unwrap_err();
        assert!(err.contains("-r"));
    }

    #[test]
    fn test_parse_many_empty_is_error() {
        assert!(Routine::parse(Vec::<&str>::new()).is_err());
        assert!(Routine::parse([""]).is_err());
        assert!(Routine::parse(["   "]).is_err());
    }
}
