use super::ExecutionStrategy;
use crate::process::ExecutionSummary;
use crate::routine::Routine;
use std::io;

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
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
                    if success {
                        summary.increment_success();
                    } else if !output.stderr.is_empty() {
                        println!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
