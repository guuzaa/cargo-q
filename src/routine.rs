use std::convert::From;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::process::{Command, Output, Stdio};

#[derive(Debug, Default, Clone)]
pub struct Routine {
    pub name: String,
    pub args: Vec<String>,
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

impl<T: AsRef<str>> From<T> for Routine {
    fn from(cmd: T) -> Self {
        let cmd: &str = cmd.as_ref();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return Routine::default();
        }

        Routine {
            name: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
        }
    }
}
