use super::{num_cpus, run_one, Report, Strategy};
use crate::executor::Options;
use crate::process::{self, Termination};
use crate::progress::Progress;
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io;
use std::sync::Arc;

pub struct Parallel;

impl Strategy for Parallel {
    /// Run every routine at once, as far as the thread pool allows.
    ///
    /// Stopping early can only mean "start nothing more": a command that is
    /// already running is left to finish, since killing it would leave a
    /// half-written target directory behind. With more routines than threads,
    /// a failure therefore still skips whatever is left in the queue.
    fn execute(
        &self,
        routines: &[Routine],
        progress: &Arc<dyn Progress>,
        options: Options,
    ) -> io::Result<Termination> {
        let pool = ThreadPool::new(routines.len().min(num_cpus()));
        let report = Arc::new(Report::default());

        for (id, routine) in routines.iter().enumerate() {
            let progress = Arc::clone(progress);
            let report = Arc::clone(&report);
            let routine = routine.clone();
            pool.execute(move || {
                if process::was_interrupted() || (!options.keep_going && report.failed()) {
                    return;
                }
                run_one(id, &routine, progress.as_ref(), options, &report);
            });
        }

        drop(pool);

        report.outcome()
    }
}
