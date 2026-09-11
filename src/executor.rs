use crate::process::Termination;
use crate::routine::Routine;
use crate::strategy::{Parallel, Sequential, Strategy};
use std::io;

pub(crate) struct Executor {
    parallel: bool,
    verbose: bool,
    routines: Vec<Routine>,
}

impl Executor {
    pub fn new(routines: Vec<Routine>, parallel: bool, verbose: bool) -> Self {
        Executor {
            parallel,
            verbose,
            routines,
        }
    }

    /// Run the routines with the selected strategy.
    pub fn execute(&self) -> io::Result<Termination> {
        let strategy: Box<dyn Strategy> = if self.parallel {
            Box::new(Parallel)
        } else {
            Box::new(Sequential)
        };

        strategy.execute(&self.routines, self.verbose)
    }
}
