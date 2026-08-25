use super::ExecutionStrategy;
use crate::progress::new_progress;
use crate::routine::Routine;
use std::io;

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let progress = new_progress(routines.len(), verbose);

        for (id, cmd) in routines.iter().enumerate() {
            let cmd_str = format!("Cargo {}", cmd.to_string());
            progress.task_started(id, &cmd_str);

            match cmd.run(verbose) {
                Ok((success, output)) => {
                    progress.task_finished(id, &cmd_str, success, &output.stderr);
                }
                Err(e) => {
                    progress.task_finished(id, &cmd_str, false, e.to_string().as_bytes());
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
