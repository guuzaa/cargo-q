use crate::routine::Routine;
use std::{io, thread};

const MAX_THREADS: usize = 8;

#[inline]
pub fn num_cpus() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(MAX_THREADS)
}

pub trait ExecutionStrategy {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<()>;
}

mod parallel;
mod sequential;

pub use parallel::ParallelStrategy;
pub use sequential::SequentialStrategy;
