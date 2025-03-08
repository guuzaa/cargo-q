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
        write!(f, "{}", self.name)?;
        for arg in &self.args {
            write!(f, " {}", arg)?;
        }
        Ok(())
    }
}

impl Routine {
    pub fn run(&self, verbose: bool) -> io::Result<(bool, Output)> {
        let mut cmd = Command::new("cargo");
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
