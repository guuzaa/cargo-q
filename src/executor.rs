use crate::routine::Routine;
use crate::strategy::{ExecutionStrategy, ParallelStrategy, SequentialStrategy};
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

    pub fn execute(&self) -> io::Result<()> {
        crate::process::install_interrupt_handler();

        let strategy: Box<dyn ExecutionStrategy> = match self.parallel {
            true => Box::new(ParallelStrategy),
            false => Box::new(SequentialStrategy),
        };

        strategy.execute(&self.routines, self.verbose)
    }
}
