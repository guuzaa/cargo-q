use std::io;
use std::process::{Command, Output, Stdio};

#[derive(Debug, Default)]
pub(crate) struct Routine {
    pub(crate) name: String,
    pub(crate) args: Vec<String>,
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

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}
