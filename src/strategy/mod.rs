mod parallel;
mod report;
mod sequential;

pub use parallel::Parallel;
pub use sequential::Sequential;

use crate::executor::Options;
use crate::process::Termination;
use crate::progress::Progress;
use crate::routine::Routine;
use report::{Failure, Report};
use std::io;
use std::sync::Arc;
use std::thread;

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
    ///
    /// A strategy decides only *when* each routine runs; reporting progress
    /// and deciding whether to keep going are shared (see [`run_one`]).
    fn execute(
        &self,
        routines: &[Routine],
        progress: &Arc<dyn Progress>,
        options: Options,
    ) -> io::Result<Termination>;
}

/// Run one routine, reporting progress and recording the first failure.
///
/// Returns `false` when the rest of the list must not be started: an
/// interrupt, a failed spawn, or — unless `keep_going` was asked for — a
/// command that exited non-zero.
pub(crate) fn run_one(
    id: usize,
    routine: &Routine,
    progress: &dyn Progress,
    options: Options,
    report: &Report,
) -> bool {
    let cmd = routine.to_string();
    progress.task_started(id, &cmd);

    let (success, proceed) = match routine.run(options.verbose, |data| {
        progress.task_output(id, data);
    }) {
        Ok(Termination::Success) => (true, true),
        Ok(Termination::Failure(code)) => {
            report.record(Failure::Exit(code));
            (false, options.keep_going)
        }
        Ok(Termination::Interrupted) => (false, false),
        Err(e) => {
            progress.task_output(id, e.to_string().as_bytes());
            report.record(Failure::Spawn(e));
            (false, false)
        }
    };

    progress.task_finished(id, &cmd, success);
    proceed
}
