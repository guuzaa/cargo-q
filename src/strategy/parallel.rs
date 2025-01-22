use super::{ExecutionStrategy, MAX_THREADS};
use crate::process::ExecutionSummary;
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io;
use std::sync::{Arc, Mutex};

pub struct ParallelStrategy;

impl ExecutionStrategy for ParallelStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()> {
        let summary = Arc::new(Mutex::new(ExecutionSummary::new(routines.len())));
        let pool = ThreadPool::new(routines.len().min(MAX_THREADS));

        for cmd in routines {
            let summary = Arc::clone(&summary);
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };

            let cmd = cmd.clone();
            pool.execute(move || match cmd.run(verbose) {
                Ok((success, output)) => {
                    summary.lock().unwrap().increment_execution();
                    summary.lock().unwrap().print_process(&cmd_str);
                    if success {
                        summary.lock().unwrap().increment_success();
                    } else if !output.stderr.is_empty() {
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    summary.lock().unwrap().increment_execution();
                    summary.lock().unwrap().print_process(&cmd_str);
                    eprintln!("error: Failed to execute command: {}", e);
                }
            });
        }

        Ok(())
    }
}
