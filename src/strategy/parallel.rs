use super::{num_cpus, ExecutionStrategy};
use crate::progress::new_progress;
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io;
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
                progress.task_started(id, &cmd_str);
                match cmd.run(verbose) {
                    Ok((success, output)) => {
                        progress.task_finished(id, &cmd_str, success, &output.stderr);
                    }
                    Err(e) => {
                        progress.task_finished(id, &cmd_str, false, &[]);
                        progress.log(&format!(
                            "error: {} Failed to execute command: {}",
                            cmd_str, e
                        ));
                    }
                }
            });
        }

        Ok(())
    }
}
