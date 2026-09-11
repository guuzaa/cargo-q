use crate::process::Termination;
use crate::routine::Routine;
use std::{io, thread};

const MAX_THREADS: usize = 8;

#[inline]
#[must_use]
pub fn num_cpus() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(MAX_THREADS)
}

pub trait Strategy {
    /// Run every routine and report the first failure, so callers can turn a
    /// failed command into a non-zero exit status.
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<Termination>;
}

mod parallel;
mod sequential;

pub use parallel::Parallel;
pub use sequential::Sequential;
