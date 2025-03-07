use super::ExecutionStrategy;
use crate::process::ExecutionSummary;
use crate::routine::Routine;
use std::io::{self, Error, ErrorKind};

pub struct DependentStrategy;

impl ExecutionStrategy for DependentStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let total_commands = routines.len();
        let mut summary = ExecutionSummary::new(total_commands);

        for cmd in routines.iter() {
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };
            summary.print_process(&cmd_str);

            match cmd.run(verbose) {
                Ok((success, output)) => {
                    if !success {
                        if !output.stderr.is_empty() {
                            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("Command failed: {}", cmd_str),
                        ));
                    }
                    summary.increment_success();
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
