//! Progress reporting for a "fancy" console, with progress bar etc.

use super::{print_summary, Progress};
use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Currently running command, as tracked for progress updates.
struct Task {
    id: usize,
    /// When the task started running.
    start: Instant,
    /// Status message for the task.
    message: String,
}

/// Progress implementation for "fancy" console, with progress bar etc.
/// Each time it prints, it clears from the cursor to the end of the console,
/// prints the status text, and then moves the cursor back up to the start
/// position. This means on errors etc. we can clear any status by clearing
/// the console too.
pub struct FancyConsoleProgress {
    state: Arc<Mutex<FancyState>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

/// Screen updates happen after this duration passes, to reduce the amount
/// of printing in the case of rapid updates. This helps with terminal flicker.
const UPDATE_DELAY: Duration = Duration::from_millis(50);

/// If there are no updates for this duration, the progress will print anyway.
/// This lets the progress show ticking timers for long-running tasks so things
/// do not appear hung.
const TIMEOUT_DELAY: Duration = Duration::from_millis(500);

impl FancyConsoleProgress {
    pub fn new(total: usize) -> Self {
        let dirty_cond = Arc::new(Condvar::new());
        let state = Arc::new(Mutex::new(FancyState {
            done: false,
            pending: Vec::new(),
            dirty: false,
            dirty_cond: dirty_cond.clone(),
            total,
            done_count: 0,
            failed_count: 0,
            started_count: 0,
            tasks: VecDeque::new(),
            start_time: Instant::now(),
        }));

        // Thread to debounce status updates -- waits a bit, then prints after
        // any dirty state.
        let thread = std::thread::spawn({
            let state_lock = state.clone();
            move || loop {
                // Wait to be notified of a display update or timeout.
                {
                    let (state, _) = dirty_cond
                        .wait_timeout_while(
                            state_lock.lock().unwrap(),
                            TIMEOUT_DELAY - UPDATE_DELAY,
                            |state| !state.done && !state.dirty,
                        )
                        .unwrap();
                    if state.done {
                        let mut out = std::io::stdout();
                        out.write_all(&state.pending).unwrap();
                        out.flush().unwrap();
                        break;
                    }
                }

                // Delay a little bit in case more display updates come in.
                // We know .dirty will only ever be cleared below, so we
                // can drop the lock here while we sleep.
                std::thread::sleep(UPDATE_DELAY);

                state_lock.lock().unwrap().print_progress();
            }
        });

        FancyConsoleProgress {
            state,
            thread: Some(thread),
        }
    }
}

impl Progress for FancyConsoleProgress {
    fn task_started(&self, id: usize, cmd: &str) {
        self.state.lock().unwrap().task_started(id, cmd);
    }

    fn task_finished(&self, id: usize, cmd: &str, success: bool, stderr: &[u8]) {
        self.state
            .lock()
            .unwrap()
            .task_finished(id, cmd, success, stderr);
    }

    fn log(&self, msg: &str) {
        self.state.lock().unwrap().log(msg);
    }
}

impl Drop for FancyConsoleProgress {
    fn drop(&mut self) {
        let (success, started, total, start_time) = {
            let mut state = self.state.lock().unwrap();
            state.cleanup();
            (
                state.done_count,
                state.started_count,
                state.total,
                state.start_time,
            )
        };
        self.thread.take().unwrap().join().unwrap();
        print_summary(success, started, total, start_time);
    }
}

struct FancyState {
    done: bool,

    /// Text to print on the next update.
    /// Typically starts with the "clear any existing progress bar" sequence.
    pending: Vec<u8>,

    /// True when there is new progress to display.
    /// When set, will notify dirty_cond.
    dirty: bool,
    dirty_cond: Arc<Condvar>,

    total: usize,
    done_count: usize,
    failed_count: usize,
    started_count: usize,
    /// Commands that are currently executing.
    /// Pushed to as tasks are started, so it's always in order of age.
    tasks: VecDeque<Task>,
    start_time: Instant,
}

impl FancyState {
    fn dirty(&mut self) {
        self.dirty = true;
        self.dirty_cond.notify_one();
    }

    fn task_started(&mut self, id: usize, cmd: &str) {
        self.started_count += 1;
        self.tasks.push_back(Task {
            id,
            start: Instant::now(),
            message: cmd.to_string(),
        });
        self.dirty();
    }

    fn task_finished(&mut self, id: usize, cmd: &str, success: bool, stderr: &[u8]) {
        self.tasks
            .remove(self.tasks.iter().position(|t| t.id == id).unwrap());

        if success {
            self.done_count += 1;
            self.dirty();
            return;
        }

        self.failed_count += 1;
        let buf = &mut self.pending;
        writeln!(buf, "failed: {}", cmd).ok();
        if !stderr.is_empty() {
            buf.extend_from_slice(stderr);
            if !stderr.ends_with(b"\n") {
                buf.push(b'\n');
            }
        }
        self.dirty();
    }

    fn log(&mut self, msg: &str) {
        self.pending.extend_from_slice(msg.as_bytes());
        self.pending.push(b'\n');
        self.dirty();
    }

    fn cleanup(&mut self) {
        self.done = true;
        self.dirty(); // let thread print final time
    }

    fn print_progress(&mut self) {
        let failed = self.failed_count;
        let completed = self.done_count + failed;
        let running = self.tasks.len();
        let buf = &mut self.pending;
        write!(
            buf,
            "[{}] {}/{} done, ",
            progress_bar(completed, running, self.total, 40),
            completed,
            self.total
        )
        .ok();
        if failed > 0 {
            write!(buf, "{} failed, ", failed).ok();
        }
        writeln!(buf, "{} running", running).ok();
        let mut lines = 1;

        let max_cols = get_cols();
        let max_tasks = 8;
        let now = Instant::now();
        for task in self.tasks.iter().take(max_tasks) {
            let delta = now.duration_since(task.start).as_secs() as usize;
            writeln!(buf, "{}", task_message(&task.message, delta, max_cols)).ok();
            lines += 1;
        }

        if self.tasks.len() > max_tasks {
            let remaining = self.tasks.len() - max_tasks;
            writeln!(buf, "...and {} more", remaining).ok();
            lines += 1;
        }

        // Move cursor up to the first printed line, for overprinting.
        write!(buf, "\x1b[{}A", lines).ok();
        let mut out = std::io::stdout();
        out.write_all(buf).unwrap();
        out.flush().unwrap();

        // Set up buf for next print.
        // If the user hit ctl-c, it may have printed something on the line.
        // So \r to go to first column first, then clear anything below.
        buf.clear();
        buf.extend_from_slice(b"\r\x1b[J");

        self.dirty = false;
    }
}

/// Format a task's status message to optionally include how long it has been running
/// and also to fit within a maximum number of terminal columns.
fn task_message(message: &str, seconds: usize, max_cols: usize) -> String {
    let time_note = if seconds > 2 {
        format!(" ({}s)", seconds)
    } else {
        String::new()
    };
    let mut out = message.to_owned();
    if out.len() + time_note.len() >= max_cols {
        let keep = max_cols.saturating_sub(time_note.len() + 3);
        out = truncate(&out, keep).to_string();
        out.push_str("...");
    }
    out.push_str(&time_note);
    out
}

fn truncate(s: &str, mut max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        while !s.is_char_boundary(max) {
            max -= 1;
        }
        &s[..max]
    }
}

/// Render completed/running/pending counts as an ASCII progress bar.
fn progress_bar(completed: usize, running: usize, total: usize, bar_size: usize) -> String {
    let mut bar = String::with_capacity(bar_size);
    let mut sum: usize = 0;
    if total == 0 {
        return " ".repeat(bar_size);
    }
    let pending = total.saturating_sub(completed + running);
    for (count, ch) in [(completed, '='), (running, '-'), (pending, ' ')] {
        sum += count;
        let mut target_size = sum * bar_size / total;
        if count > 0 && target_size == bar.len() && target_size < bar_size {
            // Special case: for non-zero count, ensure we always get at least
            // one tick.
            target_size += 1;
        }
        while bar.len() < target_size {
            bar.push(ch);
        }
    }
    bar
}

fn get_cols() -> usize {
    ioctl_cols().filter(|&n| n >= 10).unwrap_or(80)
}

#[cfg(unix)]
fn ioctl_cols() -> Option<usize> {
    use std::os::raw::{c_int, c_ulong};
    use std::os::unix::io::AsRawFd;

    #[repr(C)]
    struct Winsize {
        ws_row: u16,
        ws_col: u16,
        ws_xpixel: u16,
        ws_ypixel: u16,
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd"
    ))]
    const TIOCGWINSZ: c_ulong = 0x4008_7468;
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const TIOCGWINSZ: c_ulong = 0x5413;
    #[cfg(not(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "linux",
        target_os = "android"
    )))]
    const TIOCGWINSZ: c_ulong = 0;

    if TIOCGWINSZ == 0 {
        return None;
    }

    extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }

    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let fd = std::io::stdout().as_raw_fd();
    let ret = unsafe { ioctl(fd, TIOCGWINSZ, &mut ws) };
    if ret < 0 || ws.ws_col == 0 {
        None
    } else {
        Some(ws.ws_col as usize)
    }
}

#[cfg(not(unix))]
fn ioctl_cols() -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_rendering() {
        // Don't crash if we show progress before having any tasks.
        assert_eq!(progress_bar(0, 0, 0, 10), "          ");

        // All pending.
        assert_eq!(progress_bar(0, 0, 100, 10), "          ");

        // Half running, half pending.
        assert_eq!(progress_bar(0, 50, 100, 10), "-----     ");

        // One completed, rest mixed.
        assert_eq!(progress_bar(1, 49, 100, 10), "=----     ");

        // All but one completed-or-running, one pending.
        assert_eq!(progress_bar(1, 98, 100, 10), "=-------- ");

        // Nothing pending.
        assert_eq!(progress_bar(1, 99, 100, 10), "=---------");
    }

    #[test]
    fn task_rendering() {
        assert_eq!(task_message("building foo.o", 0, 80), "building foo.o");
        assert_eq!(task_message("building foo.o", 0, 10), "buildin...");
        assert_eq!(task_message("building foo.o", 0, 5), "bu...");
    }

    #[test]
    fn task_rendering_with_time() {
        assert_eq!(task_message("building foo.o", 5, 80), "building foo.o (5s)");
        assert_eq!(task_message("building foo.o", 5, 10), "bu... (5s)");
    }

    #[test]
    fn truncate_utf8() {
        let text = "utf8 progress bar: ━━━━━━━━━━━━";
        for len in 10..text.len() {
            // test passes if this doesn't panic
            truncate(text, len);
        }
    }
}
