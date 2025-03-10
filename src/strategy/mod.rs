use crate::routine::Routine;
use std::io;

const MAX_THREADS: usize = 8;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Strategy {
    /// Commands are independent (space separator)
    Independent,
    /// Commands pipe output to next command (> separator)
    Pipe,
}

pub trait ExecutionStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()>;
}

mod parallel;
mod sequential;

pub use parallel::ParallelStrategy;
pub use sequential::SequentialStrategy;
