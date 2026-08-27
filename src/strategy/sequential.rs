use super::ExecutionStrategy;
use crate::process::{self, Termination};
use crate::progress::new_progress;
use crate::routine::Routine;
use std::io::{self, ErrorKind};

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let progress = new_progress(routines.len(), verbose);

        for (id, cmd) in routines.iter().enumerate() {
            if process::was_interrupted() {
                return Err(interrupted());
            }

            let cmd_str = cmd.to_string();
            progress.task_started(id, &cmd_str);

            match cmd.run(verbose, |data| progress.task_output(id, data)) {
                Ok(Termination::Success) => {
                    progress.task_finished(id, &cmd_str, true);
                }
                Ok(Termination::Failure) => {
                    progress.task_finished(id, &cmd_str, false);
                }
                Ok(Termination::Interrupted) => {
                    progress.task_finished(id, &cmd_str, false);
                    return Err(interrupted());
                }
                Err(e) => {
                    progress.task_output(id, e.to_string().as_bytes());
                    progress.task_finished(id, &cmd_str, false);
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}

#[inline]
fn interrupted() -> io::Error {
    io::Error::new(ErrorKind::Interrupted, "interrupted")
}
