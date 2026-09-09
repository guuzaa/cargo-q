use super::ExecutionStrategy;
use crate::process::{self, Termination};
use crate::progress::new_progress;
use crate::routine::Routine;
use std::io;

pub struct SequentialStrategy;

impl ExecutionStrategy for SequentialStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<Termination> {
        let progress = new_progress(routines.len(), verbose);
        let mut failure = None;

        for (id, cmd) in routines.iter().enumerate() {
            if process::was_interrupted() {
                return Ok(Termination::Interrupted);
            }

            let cmd_str = cmd.to_string();
            progress.task_started(id, &cmd_str);

            match cmd.run(verbose, |data| progress.task_output(id, data)) {
                Ok(Termination::Success) => {
                    progress.task_finished(id, &cmd_str, true);
                }
                Ok(Termination::Failure(code)) => {
                    failure.get_or_insert(code);
                    progress.task_finished(id, &cmd_str, false);
                }
                Ok(Termination::Interrupted) => {
                    progress.task_finished(id, &cmd_str, false);
                    return Ok(Termination::Interrupted);
                }
                Err(e) => {
                    progress.task_output(id, e.to_string().as_bytes());
                    progress.task_finished(id, &cmd_str, false);
                    return Err(e);
                }
            }
        }

        Ok(match failure {
            Some(code) => Termination::Failure(code),
            None => Termination::Success,
        })
    }
}
