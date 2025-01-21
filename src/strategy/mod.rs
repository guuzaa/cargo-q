use crate::routine::Routine;
use std::io;

const MAX_THREADS: usize = 8;

pub trait ExecutionStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()>;
}

mod dependent;
mod parallel;
mod sequential;

pub use dependent::DependentStrategy;
pub use parallel::ParallelStrategy;
pub use sequential::SequentialStrategy;
