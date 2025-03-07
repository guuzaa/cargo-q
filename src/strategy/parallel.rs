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
        let total_commands = routines.len();
        let pool = ThreadPool::new(total_commands.min(MAX_THREADS));

        for cmd in routines.iter() {
            let summary = Arc::clone(&summary);
            let cmd_str = if cmd.args.is_empty() {
                cmd.name.clone()
            } else {
                format!("{} {}", cmd.name, cmd.args.join(" "))
            };

            let cmd = cmd.clone();
            pool.execute(move || {
                let ret = cmd.run(verbose);
                summary.lock().unwrap().print_process(&cmd_str);
                match ret {
                    Ok((success, output)) => {
                        if success {
                            summary.lock().unwrap().increment_success();
                        } else if !output.stderr.is_empty() {
                            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {} Failed to execute command: {}", cmd_str, e);
                    }
                }
            });
        }

        Ok(())
    }
}
