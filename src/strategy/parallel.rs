use super::{num_cpus, Strategy};
use crate::process::{self, Termination};
use crate::progress;
use crate::routine::Routine;
use crate::thread_pool::ThreadPool;
use std::io;
use std::sync::{Arc, Mutex, PoisonError};

pub struct Parallel;

/// The first thing that went wrong, kept for the final report. Commands run
/// concurrently, so "first" means whichever reported first, not the first in
/// command-line order.
enum Failure {
    Exit(i32),
    Spawn(io::Error),
}

impl Strategy for Parallel {
    fn execute(&self, routines: &[Routine], verbose: bool) -> io::Result<Termination> {
        let progress = progress::new(routines.len(), verbose);
        let pool = ThreadPool::new(routines.len().min(num_cpus()));
        let failure: Arc<Mutex<Option<Failure>>> = Arc::new(Mutex::new(None));

        for (id, cmd) in routines.iter().enumerate() {
            let progress = Arc::clone(&progress);
            let failure = Arc::clone(&failure);
            let cmd_str = cmd.to_string();
            let cmd = cmd.clone();
            pool.execute(move || {
                if process::was_interrupted() {
                    return;
                }
                progress.task_started(id, &cmd_str);
                match cmd.run(verbose, |data| progress.task_output(id, data)) {
                    Ok(Termination::Success) => {
                        progress.task_finished(id, &cmd_str, true);
                    }
                    Ok(Termination::Failure(code)) => {
                        record(&failure, Failure::Exit(code));
                        progress.task_finished(id, &cmd_str, false);
                    }
                    Ok(Termination::Interrupted) => {
                        progress.task_finished(id, &cmd_str, false);
                    }
                    Err(e) => {
                        progress.task_output(id, e.to_string().as_bytes());
                        progress.task_finished(id, &cmd_str, false);
                        record(&failure, Failure::Spawn(e));
                    }
                }
            });
        }

        drop(pool);

        if process::was_interrupted() {
            return Ok(Termination::Interrupted);
        }

        let failure = failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        match failure {
            Some(Failure::Exit(code)) => Ok(Termination::Failure(code)),
            Some(Failure::Spawn(e)) => Err(e),
            None => Ok(Termination::Success),
        }
    }
}

fn record(slot: &Mutex<Option<Failure>>, failure: Failure) {
    let mut slot = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.is_none() {
        *slot = Some(failure);
    }
}
