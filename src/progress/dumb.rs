//! Progress reporting for a "dumb" console, without any overprinting.

use super::{append_stream, print_summary, ColorExt, Progress};
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

/// Progress implementation for "dumb" console, without any overprinting.
pub struct DumbConsoleProgress {
    state: Mutex<DumbState>,
}

struct DumbState {
    verbose: bool,
    total: usize,
    success_count: usize,
    started_count: usize,
    start_time: Instant,
    /// Captured merged output per command. Used when not verbose.
    outputs: Vec<Vec<u8>>,
}

impl DumbConsoleProgress {
    pub fn new(total: usize, verbose: bool) -> Self {
        Self {
            state: Mutex::new(DumbState {
                verbose,
                total,
                success_count: 0,
                started_count: 0,
                start_time: Instant::now(),
                outputs: vec![Vec::new(); total],
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

    fn task_output(&self, id: usize, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let mut state = self.state.lock().unwrap();
        if state.verbose {
            let mut out = std::io::stdout();
            let _ = out.write_all(data);
            let _ = out.flush();
            return;
        }
        if let Some(buf) = state.outputs.get_mut(id) {
            buf.extend_from_slice(data);
        }
    }

    fn task_finished(&self, id: usize, cmd: &str, success: bool) {
        let mut state = self.state.lock().unwrap();
        if success {
            state.success_count += 1;
            if let Some(buf) = state.outputs.get_mut(id) {
                buf.clear();
            }
            return;
        }

        let output = state
            .outputs
            .get_mut(id)
            .map(std::mem::take)
            .unwrap_or_default();
        if state.verbose {
            println!("failed: {}", cmd);
            return;
        }

        let head = format!("failed: {}", cmd);
        let mut buf = Vec::with_capacity(head.len() + output.len());
        append_stream(&mut buf, head.as_bytes());
        append_stream(&mut buf, &output);
        print!("{}", String::from_utf8_lossy(&buf));
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
    fn tracks_count() {
        let progress = DumbConsoleProgress::new(4, false);
        assert_eq!(progress.success_count(), 0);
        assert_eq!(progress.total(), 4);

        progress.task_started(0, "check");
        progress.task_finished(0, "check", true);
        assert_eq!(progress.success_count(), 1);

        progress.task_started(1, "test");
        progress.task_finished(1, "test", true);
        assert_eq!(progress.success_count(), 2);

        progress.task_started(2, "run");
        progress.task_finished(2, "run", true);
        assert_eq!(progress.success_count(), 3);

        progress.task_started(3, "fmt");
        progress.task_output(3, b"error");
        progress.task_finished(3, "fmt", false);
        assert_eq!(progress.success_count(), 3);
    }
}
