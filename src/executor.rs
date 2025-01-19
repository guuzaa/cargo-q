use crate::parser::Strategy;
use crate::process::{ColorExt, ExecutionSummary};
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io::{self, Error, ErrorKind};
use std::sync::{Arc, Mutex};

const MAX_THREADS: usize = 8;

pub(crate) struct Executor {
    pub(super) parallel: bool,
    pub(super) verbose: bool,
    pub(super) strategy: Strategy,
    pub(super) routines: Vec<Routine>,
}

impl Executor {
    pub fn new(parallel: bool, verbose: bool, routines: Vec<Routine>, strategy: Strategy) -> Self {
        Executor {
            parallel,
            verbose,
            strategy,
            routines,
        }
    }

    pub fn execute(&self) -> io::Result<()> {
        match (self.parallel, self.strategy) {
            (true, Strategy::Independent) => self.execute_parallel(),
            (true, _) => Err(Error::new(
                ErrorKind::InvalidInput,
                "Parallel execution only supports independent commands now",
            )),
            (false, _) => self.execute_sequential(),
        }
    }

    fn execute_parallel(&self) -> io::Result<()> {
        let summary = Arc::new(Mutex::new(ExecutionSummary::new(self.routines.len())));
        let total_commands = self.routines.len();
        let pool = ThreadPool::new(total_commands.min(MAX_THREADS));

        for (idx, cmd) in self.routines.iter().enumerate() {
            let summary = Arc::clone(&summary);
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

            let cmd = cmd.clone();
            let verbose = self.verbose;

            pool.execute(move || match cmd.run(verbose) {
                Ok((success, output)) => {
                    if success {
                        summary.lock().unwrap().increment_success();
                    } else if !output.stderr.is_empty() {
                        eprintln!("error: Command failed");
                        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    }
                }
                Err(e) => {
                    eprintln!("error: Failed to execute command: {}", e);
                }
            });
        }

        // Pool will be dropped here, which waits for all jobs to complete
        Ok(())
    }

    fn execute_sequential(&self) -> io::Result<()> {
        let total_commands = self.routines.len();
        let mut summary = ExecutionSummary::new(total_commands);

        for (idx, cmd) in self.routines.iter().enumerate() {
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

            match cmd.run(self.verbose) {
                Ok((success, output)) => {
                    match self.strategy {
                        Strategy::Independent => {
                            if success {
                                summary.increment_success();
                            } else if !output.stderr.is_empty() {
                                eprintln!("error: Command failed but continuing due to Independent strategy");
                                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                            }
                        }
                        Strategy::Dependent | Strategy::Pipe => {
                            if !success {
                                if !output.stderr.is_empty() {
                                    eprintln!("error: {}", String::from_utf8_lossy(&output.stderr));
                                }
                                return Err(Error::new(
                                    ErrorKind::Other,
                                    format!("Command failed: cargo {}", cmd_str),
                                ));
                            }
                            summary.increment_success();
                        }
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
