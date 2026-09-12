use crate::process::{self, Termination};
use std::io;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The first thing that went wrong, kept for the final report.
pub(crate) enum Failure {
    Exit(i32),
    Spawn(io::Error),
}

/// The first failure seen while running a list of routines.
///
/// Both strategies share this: the parallel one records into it from several
/// worker threads, and the sequential one uses the same type without ever
/// contending for the lock. "First" means the first *reported*, which in
/// parallel mode is not necessarily the first in command-line order.
#[derive(Default)]
pub(crate) struct Report {
    failure: Mutex<Option<Failure>>,
}

impl Report {
    fn lock(&self) -> MutexGuard<'_, Option<Failure>> {
        self.failure.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Record `failure`, keeping whichever arrived first.
    pub(crate) fn record(&self, failure: Failure) {
        let mut slot = self.lock();
        if slot.is_none() {
            *slot = Some(failure);
        }
    }

    /// Whether a command has already failed.
    pub(crate) fn failed(&self) -> bool {
        self.lock().is_some()
    }

    /// The outcome to hand back to the caller.
    ///
    /// An interrupt outranks a failure: a command killed by Ctrl-C also leaves
    /// a non-zero status behind, but what the user asked for was to stop.
    pub(crate) fn outcome(&self) -> io::Result<Termination> {
        if process::was_interrupted() {
            return Ok(Termination::Interrupted);
        }
        match self.lock().take() {
            Some(Failure::Exit(code)) => Ok(Termination::Failure(code)),
            Some(Failure::Spawn(e)) => Err(e),
            None => Ok(Termination::Success),
        }
    }
}
