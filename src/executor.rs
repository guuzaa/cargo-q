use crate::routine::Routine;
use crate::strategy::{ExecutionStrategy, ParallelStrategy, SequentialStrategy};
use std::io;

pub(crate) struct Executor {
    pub(super) parallel: bool,
    pub(super) verbose: bool,
    pub(super) routines: Vec<Routine>,
}

impl Executor {
    pub fn new(parallel: bool, verbose: bool, routines: Vec<Routine>) -> Self {
        Executor {
            parallel,
            verbose,
            routines,
        }
    }

    pub fn execute(&self) -> io::Result<()> {
        let strategy: Box<dyn ExecutionStrategy> = match self.parallel {
            true => Box::new(ParallelStrategy),
            false => Box::new(SequentialStrategy),
        };

        strategy.execute(&self.routines, self.verbose)
    }
}
