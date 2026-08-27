use super::{num_cpus, ExecutionStrategy};
use crate::process::{self, Termination};
use crate::progress::new_progress;
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io::{self, ErrorKind};
use std::sync::Arc;

pub struct ParallelStrategy;

impl ExecutionStrategy for ParallelStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let progress = new_progress(routines.len(), verbose);
        let pool = ThreadPool::new(routines.len().min(num_cpus()));

        for (id, cmd) in routines.iter().enumerate() {
            let progress = Arc::clone(&progress);
            let cmd_str = cmd.to_string();
            let cmd = cmd.clone();
            pool.execute(move || {
                if process::was_interrupted() {
                    return;
                }
                progress.task_started(id, &cmd_str);
                match cmd.run(verbose, |data| progress.task_output(id, data)) {
                    Ok(Termination::Success) => {
                        progress.task_finished(id, &cmd_str, true);
                    }
                    Ok(Termination::Failure | Termination::Interrupted) => {
                        progress.task_finished(id, &cmd_str, false);
                    }
                    Err(e) => {
                        progress.task_output(id, e.to_string().as_bytes());
                        progress.task_finished(id, &cmd_str, false);
                    }
                }
            });
        }

        drop(pool);

        if process::was_interrupted() {
            Err(io::Error::new(ErrorKind::Interrupted, "interrupted"))
        } else {
            Ok(())
        }
    }
}
