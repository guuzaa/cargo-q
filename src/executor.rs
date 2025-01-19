use crate::parser::Strategy;
use crate::process::{ColorExt, ExecutionSummary};
use crate::routine::Routine;
use std::io::{self, Error, ErrorKind};

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
