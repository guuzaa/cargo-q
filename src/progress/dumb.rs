//! Progress reporting for a "dumb" console, without any overprinting.

use super::{append_stream, print_summary, write_tail, Colored, Progress, Tail};
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::Instant;

/// Progress implementation for "dumb" console, without any overprinting.
pub struct ConsoleProgress {
    verbose: bool,
    state: Mutex<DumbState>,
    outputs: Vec<Mutex<Tail>>,
}

struct DumbState {
    total: usize,
    success_count: usize,
    started_count: usize,
    start_time: Instant,
}

impl ConsoleProgress {
    pub fn new(total: usize, verbose: bool) -> Self {
        Self {
            verbose,
            state: Mutex::new(DumbState {
                total,
                success_count: 0,
                started_count: 0,
                start_time: Instant::now(),
            }),
            outputs: (0..total).map(|_| Mutex::new(Tail::new())).collect(),
        }
    }
}

impl Progress for ConsoleProgress {
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
        if self.verbose {
            let mut out = io::stdout().lock();
            let _ = out.write_all(data);
            let _ = out.flush();
            return;
        }
        if let Some(buf) = self.outputs.get(id) {
            buf.lock().unwrap().push(data);
        }
    }

    fn task_finished(&self, id: usize, cmd: &str, success: bool) {
        if success {
            self.state.lock().unwrap().success_count += 1;
            if let Some(buf) = self.outputs.get(id) {
                buf.lock().unwrap().clear();
            }
            return;
        }

        if self.verbose {
            println!("failed: {cmd}");
            return;
        }

        let tail = self
            .outputs
            .get(id)
            .map(|buf| buf.lock().unwrap().take())
            .unwrap_or_else(Tail::new);
        let head = format!("failed: {cmd}");
        let mut buf = Vec::with_capacity(head.len() + 32);
        append_stream(&mut buf, head.as_bytes());
        write_tail(&mut buf, &tail);
        print!("{}", String::from_utf8_lossy(&buf));
    }
}

impl Drop for ConsoleProgress {
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

    impl ConsoleProgress {
        fn success_count(&self) -> usize {
            self.state.lock().unwrap().success_count
        }

        fn total(&self) -> usize {
            self.state.lock().unwrap().total
        }
    }

    #[test]
    fn tracks_count() {
        let progress = ConsoleProgress::new(4, false);
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
