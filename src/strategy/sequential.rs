use super::ExecutionStrategy;
use crate::process::ExecutionSummary;
use crate::routine::Routine;
use std::io;

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let mut summary = ExecutionSummary::new(routines.len());

        for cmd in routines {
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };

            summary.increment_execution();
            summary.print_process(&cmd_str);

            match cmd.run(verbose) {
                Ok((success, output)) => {
                    if success {
                        summary.increment_success();
                    } else if !output.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
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
