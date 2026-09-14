use super::{num_cpus, run_one, Report, Strategy};
use crate::executor::Options;
use crate::process::{self, Termination};
use crate::progress::Progress;
use crate::routine::Routine;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Parallel;

impl Strategy for Parallel {
    /// Run every routine at once, as far as the worker count allows.
    ///
    /// Stopping early can only mean "start nothing more": a command that is
    /// already running is left to finish, since killing it would leave a
    /// half-written target directory behind. With more routines than workers,
    /// a failure therefore still skips whatever is left in the queue.
    fn execute(
        &self,
        routines: &[Routine],
        progress: &Arc<dyn Progress>,
        options: Options,
    ) -> io::Result<Termination> {
        let report = Report::default();
        let workers = routines.len().min(num_cpus());
        if workers == 0 {
            return report.outcome();
        }

        let next = AtomicUsize::new(0);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= routines.len() {
                        break;
                    }
                    if process::was_interrupted() || (!options.keep_going && report.failed()) {
                        next.store(routines.len(), Ordering::Relaxed);
                        break;
                    }
                    run_one(i, &routines[i], progress.as_ref(), options, &report);
                });
            }
        });

        report.outcome()
    }
}
