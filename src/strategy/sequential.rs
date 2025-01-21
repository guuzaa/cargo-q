use super::ExecutionStrategy;
use crate::process::{ColorExt, ExecutionSummary};
use crate::routine::Routine;
use std::io;

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let total_commands = routines.len();
        let mut summary = ExecutionSummary::new(total_commands);

        for (idx, cmd) in routines.iter().enumerate() {
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };
            println!(
                "\n    {} {}",
                format!("[{}/{}]", idx + 1, total_commands).bold(),
                cmd_str
            );

            match cmd.run(verbose) {
                Ok((success, output)) => {
                    if success {
                        summary.increment_success();
                    } else if !output.stderr.is_empty() {
                        eprintln!(
                            "error: Command failed but continuing due to Independent strategy"
                        );
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
