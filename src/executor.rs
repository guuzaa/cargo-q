use crate::parser::Strategy;
use crate::routine::Routine;
use std::io::{self, Error, ErrorKind};
use std::time::Instant;

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
        if self.parallel {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Parallel execution not yet implemented",
            ));
        }

        let start_time = Instant::now();
        let mut success_count = 0;
        let total_commands = self.routines.len();

        for cmd in &self.routines {
            let success = cmd.run(self.verbose)?;

            match self.strategy {
                Strategy::Independent => {
                    // Continue to next command regardless of success
                    if success {
                        success_count += 1;
                    }
                }
                Strategy::Dependent | Strategy::Pipe => {
                    // Stop if command failed
                    if !success {
                        let elapsed = start_time.elapsed();
                        eprintln!(
                            "Summary: {}/{} commands succeeded ({:.2}s)",
                            success_count,
                            total_commands,
                            elapsed.as_secs_f32()
                        );
                        return Err(Error::new(
                            ErrorKind::Other,
                            format!("Command 'cargo {} {}' failed", cmd.name, cmd.args.join(" ")),
                        ));
                    }
                    success_count += 1;
                }
            }
        }

        let elapsed = start_time.elapsed();
        println!(
            "Summary: {}/{} commands succeeded ({:.2}s)",
            success_count,
            total_commands,
            elapsed.as_secs_f32()
        );
        Ok(())
    }
}
