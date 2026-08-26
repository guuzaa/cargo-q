//! Progress reporting for a "dumb" console, without any overprinting.

use super::{print_summary, ColorExt, Progress};
use std::sync::Mutex;
use std::time::Instant;

/// Progress implementation for "dumb" console, without any overprinting.
pub struct DumbConsoleProgress {
    state: Mutex<DumbState>,
}

struct DumbState {
    total: usize,
    success_count: usize,
    started_count: usize,
    start_time: Instant,
}

impl DumbConsoleProgress {
    pub fn new(total: usize) -> Self {
        Self {
            state: Mutex::new(DumbState {
                total,
                success_count: 0,
                started_count: 0,
                start_time: Instant::now(),
            }),
        }
    }
}

impl Progress for DumbConsoleProgress {
    fn task_started(&self, _id: usize, cmd: &str) {
        let mut state = self.state.lock().unwrap();
        state.started_count += 1;
        println!(
            "{} {}",
            format!("[{}/{}]", state.started_count, state.total).bold(),
            cmd
        );
    }

    fn task_finished(&self, _id: usize, _cmd: &str, success: bool, stderr: &[u8]) {
        let mut state = self.state.lock().unwrap();
        if success {
            state.success_count += 1;
        } else if !stderr.is_empty() {
            println!("{}", String::from_utf8_lossy(stderr));
        }
    }

    fn log(&self, msg: &str) {
        let _state = self.state.lock().unwrap();
        println!("{}", msg);
    }
}

impl Drop for DumbConsoleProgress {
    fn drop(&mut self) {
        let state = self.state.lock().unwrap();
        print_summary(
            state.success_count,
            state.started_count,
            state.total,
            state.start_time,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::Progress;

    impl DumbConsoleProgress {
        fn success_count(&self) -> usize {
            self.state.lock().unwrap().success_count
        }

        fn total(&self) -> usize {
            self.state.lock().unwrap().total
        }
    }

    #[test]
    fn tracks_success_count() {
        let progress = DumbConsoleProgress::new(3);
        assert_eq!(progress.success_count(), 0);
        assert_eq!(progress.total(), 3);

        progress.task_started(0, "check");
        progress.task_finished(0, "check", true, &[]);
        assert_eq!(progress.success_count(), 1);

        progress.task_started(1, "test");
        progress.task_finished(1, "test", true, &[]);
        assert_eq!(progress.success_count(), 2);

        progress.task_started(2, "run");
        progress.task_finished(2, "run", true, &[]);
        assert_eq!(progress.success_count(), 3);
    }
}
