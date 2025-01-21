use crate::routine::Routine;
use std::io;

const MAX_THREADS: usize = 8;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Strategy {
    /// Commands are independent (space or ; separator)
    Independent,
    /// Commands are dependent on previous success (& separator)
    Dependent,
    /// Commands pipe output to next command (> separator)
    Pipe,
}

pub trait ExecutionStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()>;
}

mod dependent;
mod parallel;
mod sequential;

pub use dependent::DependentStrategy;
pub use parallel::ParallelStrategy;
pub use sequential::SequentialStrategy;
