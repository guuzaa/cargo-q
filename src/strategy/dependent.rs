use super::ExecutionStrategy;
use crate::process::ExecutionSummary;
use crate::routine::Routine;
use std::io::{self, Error, ErrorKind};

pub struct DependentStrategy;

impl ExecutionStrategy for DependentStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let mut summary = ExecutionSummary::new(routines.len());
        for cmd in routines {
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };

            match cmd.run(verbose) {
                Ok((success, output)) => {
                    summary.increment_execution();
                    summary.print_process(&cmd_str);

                    if !success {
                        if !output.stderr.is_empty() {
                            eprintln!("error: {}", String::from_utf8_lossy(&output.stderr));
                        }
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("Command failed: cargo {}", cmd_str),
                        ));
                    }
                    summary.increment_success();
                }
                Err(e) => {
                    eprintln!("error: Failed to execute command: {}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
