use std::io;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub(crate) struct Routine {
    pub(crate) name: String,
    pub(crate) args: Vec<String>,
}

impl Routine {
    pub fn run(&self, verbose: bool) -> io::Result<bool> {
        let stdout = if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        };

        let stderr = if verbose {
            Stdio::inherit()
        } else {
            Stdio::null()
        };

        let status = Command::new("cargo")
            .arg(&self.name)
            .args(&self.args)
            .stdout(stdout)
            .stderr(stderr)
            .status()?;

        Ok(status.success())
    }
}
