use crate::routine::Routine;
use crate::strategy::{
    DependentStrategy, ExecutionStrategy, ParallelStrategy, SequentialStrategy, Strategy,
};
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
        let strategy: Box<dyn ExecutionStrategy> = match (self.parallel, self.strategy) {
            (true, Strategy::Independent) => Box::new(ParallelStrategy),
            (true, _) => {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Parallel execution only supports independent commands now",
                ))
            }
            (false, Strategy::Independent) => Box::new(SequentialStrategy),
            (false, Strategy::Dependent) => Box::new(DependentStrategy),
            (false, Strategy::Pipe) => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Pipe strategy not implemented yet",
                ))
            }
        };

        strategy.execute(&self.routines, self.verbose)
    }
}
