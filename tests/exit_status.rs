//! End-to-end checks on what cargo-q runs and what the shell sees.
//!
//! A failed command must reach the shell as a non-zero exit status carrying
//! the child's own code — cargo-q runs commands to report on them, so scripts
//! and CI must be able to rely on the status — and a failure must stop the
//! rest of the list unless `--keep-going` says otherwise.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output};
use std::sync::atomic::{AtomicU64, Ordering};

fn cargo_q(args: &[&str]) -> ExitStatus {
    cargo_q_output(args).status
}

fn cargo_q_output(args: &[&str]) -> Output {
    cargo_q_output_in(Path::new("."), args)
}

fn cargo_q_in(dir: impl AsRef<Path>, args: &[&str]) -> ExitStatus {
    cargo_q_output_in(dir, args).status
}

fn cargo_q_output_in(dir: impl AsRef<Path>, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(args)
        // Pass a borrow: `current_dir` takes its argument by value, so an
        // owning `dir` would be dropped — and a temporary directory deleted —
        // before the command even spawns.
        .current_dir(dir.as_ref())
        .output()
        .expect("run cargo-q")
}

/// A directory with no `Cargo.toml` in it or above, so `cargo locate-project`
/// fails with cargo's own error code (101). Removed on drop.
struct OutsidePackage(PathBuf);

impl OutsidePackage {
    fn new() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cargo-q-exit-status-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self(dir)
    }
}

impl Drop for OutsidePackage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl AsRef<Path> for OutsidePackage {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

fn outside_a_package() -> OutsidePackage {
    OutsidePackage::new()
}

#[test]
fn failing_command_exits_nonzero() {
    assert!(!cargo_q(&["definitely-not-a-cargo-command"]).success());
}

#[test]
fn failure_in_parallel_mode_exits_nonzero() {
    let status = cargo_q(&[
        "-p",
        "definitely-not-a-cargo-command",
        "locate-project",
        "--quiet",
    ]);
    assert!(!status.success());
}

#[test]
fn propagates_the_child_exit_code() {
    let status = cargo_q_in(outside_a_package(), &["locate-project", "--quiet"]);
    assert_eq!(status.code(), Some(101), "{status:?}");
}

#[test]
fn propagates_the_child_exit_code_in_parallel_mode() {
    let status = cargo_q_in(outside_a_package(), &["-p", "locate-project", "--quiet"]);
    assert_eq!(status.code(), Some(101), "{status:?}");
}

#[test]
fn successful_command_exits_zero() {
    assert!(cargo_q(&["locate-project", "--quiet"]).success());
}

/// A failure stops the run, and the summary accounts for what never started.
#[test]
fn stops_at_the_first_failure() {
    let output = cargo_q_output_in(outside_a_package(), &["locate-project --quiet", "version"]);
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 skipped"), "{stdout}");
}

#[test]
fn keep_going_runs_the_rest_of_the_list() {
    let output = cargo_q_output_in(
        outside_a_package(),
        &["-k", "locate-project --quiet", "version"],
    );
    // The failure is still what the shell sees, even though the run finished.
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 succeeded, 1 failed, 0 skipped"),
        "{stdout}"
    );
}

/// A failure reports its exit code on the line that introduces it.
#[test]
fn failure_reports_its_exit_code() {
    let output = cargo_q_output_in(outside_a_package(), &["locate-project"]);
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("failed: cargo locate-project (exit 101)"),
        "{stdout}"
    );
}

/// With `--keep-going` several commands can fail, but only the first one's
/// code reaches the shell, so each failure line carries its own.
#[test]
fn keep_going_labels_every_failure_with_its_exit_code() {
    let output = cargo_q_output_in(
        outside_a_package(),
        &[
            "-k",
            "locate-project --quiet",
            "metadata --quiet",
            "version",
        ],
    );
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.matches("(exit 101)").count(),
        2,
        "both failures carry their own code: {stdout}"
    );
    assert!(
        stdout.contains("1 succeeded, 2 failed, 0 skipped"),
        "{stdout}"
    );
}

/// A dry run spawns nothing, so a command that would fail still exits zero.
#[test]
fn dry_run_prints_the_plan_without_running_anything() {
    let output = cargo_q_output_in(
        outside_a_package(),
        &["--dry-run", "locate-project --quiet", "version"],
    );
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cargo locate-project --quiet"), "{stdout}");
    assert!(stdout.contains("cargo version"), "{stdout}");
}

/// The parser cannot always tell an argument from a subcommand name, so a dry
/// run is how to see that `--features test` grew a second command.
#[test]
fn dry_run_reveals_a_token_read_as_a_new_command() {
    let output = cargo_q_output(&["--dry-run", "build", "--features", "test"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cargo build --features"), "{stdout}");
    assert!(stdout.contains("cargo test"), "{stdout}");
}

#[test]
fn unknown_bare_token_exits_nonzero() {
    let output = cargo_q_output(&["build", "--features", "f1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is not a cargo subcommand"), "{stderr}");
}

/// A round is the whole list, so every command runs once per round.
#[test]
fn repeat_runs_the_whole_list_once_per_round() {
    let output = cargo_q_output(&["--repeat", "3", "version"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("cargo version").count(), 3, "{stdout}");
}

/// A failure ends the run, so the later rounds never start.
#[test]
fn a_failure_stops_the_later_rounds() {
    let output = cargo_q_output_in(
        outside_a_package(),
        &["--repeat", "3", "locate-project --quiet", "version"],
    );
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("cargo version").count(), 0, "{stdout}");
    // The rounds that never ran are part of the plan, so they are skipped.
    assert!(stdout.contains("5 skipped"), "{stdout}");
}

/// `--keep-going` runs the rest of the list, and the later rounds too.
#[test]
fn keep_going_runs_every_round() {
    let output = cargo_q_output_in(
        outside_a_package(),
        &["-k", "--repeat", "3", "locate-project --quiet", "version"],
    );
    // The failure is still what the shell sees, even though the run finished.
    assert_eq!(output.status.code(), Some(101), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.matches("cargo version").count(), 3, "{stdout}");
}

/// The rounds run one after another, so there is nothing for `--parallel`
/// to do.
#[test]
fn repeat_and_parallel_are_mutually_exclusive() {
    let output = cargo_q_output(&["--repeat", "3", "-p", "version"]);
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}

/// A dry run shows how many rounds the list would go through.
#[test]
fn dry_run_shows_the_rounds() {
    let output = cargo_q_output(&["--repeat", "3", "--dry-run", "check", "test"]);
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("2 command(s) x 3 rounds"), "{stdout}");
    assert!(stdout.contains("cargo check"), "{stdout}");
    assert!(stdout.contains("cargo test"), "{stdout}");
}

/// Install a `cargo` wrapper that records its pid and then stalls by replacing
/// itself with `sleep`, so cargo-q stays blocked inside name resolution for
/// `seconds`. `exec` keeps the pid and drops the shell, whose SIGINT handling
/// differs between platforms (`bash` on macOS, `dash` on Linux).
#[cfg(unix)]
fn stalling_cargo(dir: &Path, seconds: u32) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = dir.join("stalling-cargo");
    let pid_file = dir.join("stalling-cargo.pid");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho $$ > '{}'\nexec sleep {seconds}\n",
            pid_file.display()
        ),
    )
    .expect("write wrapper");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("make wrapper executable");
    pid_file
}

/// A signal that arrives while cargo-q is resolving subcommand names must end
/// in a clean `Interrupted`, not the default action and not a misleading
/// "not a cargo subcommand" error.
///
/// The wrapper exits on its own, and the signal goes straight to cargo-q via
/// `libc::kill`: `kill(1)` is known to misparse negative pids (procps on
/// Linux), which silently sent the signal to the wrong process group and made
/// this test fail on Linux with exit 1.
#[cfg(unix)]
#[test]
fn interrupt_during_name_resolution_exits_interrupted() {
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    let pid_file = stalling_cargo(&dir.0, 1);

    let child = Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(["check", "some-third-party"])
        .env("CARGO", dir.0.join("stalling-cargo"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run cargo-q");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !pid_file.exists() {
        assert!(Instant::now() < deadline, "cargo --list never started");
        std::thread::sleep(Duration::from_millis(10));
    }

    let signalled = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
    assert_eq!(signalled, 0, "send SIGINT to cargo-q");

    let output = child.wait_with_output().expect("wait for cargo-q");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(130), "{stderr}");
    assert!(!stderr.contains("is not a cargo subcommand"), "{stderr}");
}

/// SIGTERM must take the same path as Ctrl-C: the handler kills the cargo
/// process group, and cargo-q exits 130 (`Termination::Interrupted`) instead
/// of leaving that child running.
#[cfg(unix)]
#[test]
fn sigterm_kills_the_child_and_exits_interrupted() {
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    // Long enough that a missed signal cannot look like a clean exit.
    let pid_file = stalling_cargo(&dir.0, 30);

    let mut cargo_q = Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(["version"])
        .env("CARGO", dir.0.join("stalling-cargo"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run cargo-q");

    let started = Instant::now();
    while !pid_file.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "child cargo never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let worker_pid: libc::pid_t = std::fs::read_to_string(&pid_file)
        .expect("read child pid")
        .trim()
        .parse()
        .expect("child pid");
    // The child is in its own process group, so a failed test must kill it
    // itself. Disarmed only after we have seen it die.
    let mut reap_worker = KillOnDrop(Some(worker_pid));

    let signalled = unsafe { libc::kill(cargo_q.id() as libc::pid_t, libc::SIGTERM) };
    assert_eq!(signalled, 0, "send SIGTERM to cargo-q");

    let status = wait_for_exit(&mut cargo_q, Duration::from_secs(5));
    let stderr = {
        let mut buf = String::new();
        if let Some(mut err) = cargo_q.stderr.take() {
            use std::io::Read;
            err.read_to_string(&mut buf).expect("read stderr");
        }
        buf
    };
    assert_eq!(status.code(), Some(130), "{stderr}");
    assert!(
        !process_alive(worker_pid),
        "child cargo pid {worker_pid} still running after SIGTERM"
    );
    reap_worker.0 = None;
}

/// The terminal's Ctrl-C reaches a verbose child because it shares our
/// process group: the child is interrupted even though we did not forward
/// anything to it, and cargo-q knows it was interrupted.
#[cfg(target_os = "linux")]
#[test]
fn the_terminal_reaches_a_verbose_child() {
    use std::io::Write;
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    let ready_file = dir.0.join("ready");
    let interrupted_file = dir.0.join("interrupted");
    let fake = write_script(
        &dir.0,
        "interrupting-cargo",
        &format!(
            "trap 'echo interrupted > {}; exit 42' INT\n\
             echo ready > {}\n\
             sleep 5\n",
            interrupted_file.display(),
            ready_file.display()
        ),
    );

    // `script` runs the command with a pseudo terminal of its own.
    let mut script = Command::new("script")
        .args([
            "-qec",
            &format!("{} -v version", env!("CARGO_BIN_EXE_cargo-q")),
            "/dev/null",
        ])
        .env("CARGO", &fake)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("run script");

    // Wait until the child has its trap in place, then press Ctrl-C on the
    // terminal: the terminal signals the whole foreground group, which is
    // ours and the child's.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready_file.exists() {
        assert!(Instant::now() < deadline, "the child cargo never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    if let Some(stdin) = script.stdin.as_mut() {
        stdin.write_all(b"\x03").expect("press Ctrl-C");
        stdin.flush().expect("flush Ctrl-C");
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    while !interrupted_file.exists() {
        assert!(
            Instant::now() < deadline,
            "the terminal never reached the child"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let status = script.wait().expect("wait for cargo-q");
    // The child caught the interrupt and left a code of its own, but cargo-q
    // was interrupted as well, so that is what it reports.
    assert_eq!(status.code(), Some(130), "{status:?}");
}

/// A verbose child keeps our process group, so a signal sent to us alone has
/// to reach it on its own: the first interrupt asks it to stop, a later one
/// takes it down.
///
/// The children it started are out of reach that way — the price of sharing
/// the group — but the terminal's Ctrl-C reaches all of them.
#[cfg(unix)]
#[test]
fn a_direct_interrupt_stops_a_verbose_child() {
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    let child_pid_file = dir.0.join("child.pid");
    let grandchild_pid_file = dir.0.join("grandchild.pid");
    // The child ignores SIGINT, so only the kill can end it.
    let fake = write_script(
        &dir.0,
        "stubborn-cargo",
        &format!(
            "trap '' INT\n\
             echo $$ > '{}'\n\
             sh -c 'trap \"exit 42\" INT; sleep 5' &\n\
             echo $! > '{}'\n\
             wait\n",
            child_pid_file.display(),
            grandchild_pid_file.display()
        ),
    );

    let mut cargo_q = Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(["-v", "version"])
        .env("CARGO", &fake)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run cargo-q");

    let started = Instant::now();
    while !grandchild_pid_file.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the child cargo never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let child_pid: libc::pid_t = read_number(&child_pid_file);
    let grandchild_pid: libc::pid_t = read_number(&grandchild_pid_file);
    let mut reap_child = KillOnDrop(Some(child_pid));
    // Kept armed: the grandchild is still running and has to be cleaned up.
    let _reap_grandchild = KillOnDrop(Some(grandchild_pid));

    // The first interrupt asks the child to stop; it ignores SIGINT.
    let cargo_q_pid = libc::pid_t::try_from(cargo_q.id()).expect("cargo-q pid");
    let signalled = unsafe { libc::kill(cargo_q_pid, libc::SIGINT) };
    assert_eq!(signalled, 0, "send SIGINT to cargo-q");
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        process_alive(child_pid),
        "the child died even though it ignores SIGINT"
    );

    // A later one takes it down.
    let signalled = unsafe { libc::kill(cargo_q_pid, libc::SIGINT) };
    assert_eq!(signalled, 0, "send SIGINT to cargo-q again");
    let status = wait_for_exit(&mut cargo_q, Duration::from_secs(5));
    assert_eq!(status.code(), Some(130), "{status:?}");
    assert!(!process_alive(child_pid), "child cargo still running");
    // What a verbose child started shares our group, so a signal sent to us
    // alone does not reach it.
    assert!(
        process_alive(grandchild_pid),
        "the grandchild should have survived a signal sent to cargo-q"
    );
    reap_child.0 = None;
}

/// A captured child leads its own process group, so a signal sent to us
/// takes down the grandchildren cargo started as well.
#[cfg(unix)]
#[test]
fn a_direct_interrupt_kills_the_grandchildren_of_a_captured_child() {
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    let child_pid_file = dir.0.join("child.pid");
    let grandchild_pid_file = dir.0.join("grandchild.pid");
    // The grandchild runs in the foreground on purpose. A background job of a
    // non-interactive shell has SIGINT set to ignore, and a signal ignored on
    // entry to a shell cannot be trapped, so it would outlive the interrupt no
    // matter what cargo-q did with the group.
    //
    // `exec` keeps the pid, so the recorded one is the process that has to
    // die, and it dies whichever way the interrupt arrives: before the exec
    // it is a shell with the default disposition, after it a `sleep` with the
    // default disposition. A `sleep` forked by a shell would be a different
    // process, and one forked after the signal was delivered would never see
    // it.
    let fake = write_script(
        &dir.0,
        "cargo-with-a-child",
        &format!(
            "echo $$ > '{}'\n\
             sh -c 'echo $$ > \"{}\"; exec sleep 5'\n",
            child_pid_file.display(),
            grandchild_pid_file.display()
        ),
    );

    let mut cargo_q = Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(["version"])
        .env("CARGO", &fake)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run cargo-q");

    let started = Instant::now();
    while !grandchild_pid_file.exists() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the child cargo never started"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let child_pid: libc::pid_t = read_number(&child_pid_file);
    let grandchild_pid: libc::pid_t = read_number(&grandchild_pid_file);
    let mut reap_child = KillOnDrop(Some(child_pid));
    let mut reap_grandchild = KillOnDrop(Some(grandchild_pid));

    let cargo_q_pid = libc::pid_t::try_from(cargo_q.id()).expect("cargo-q pid");
    let signalled = unsafe { libc::kill(cargo_q_pid, libc::SIGINT) };
    assert_eq!(signalled, 0, "send SIGINT to cargo-q");

    let status = wait_for_exit(&mut cargo_q, Duration::from_secs(5));
    assert_eq!(status.code(), Some(130), "{status:?}");
    assert!(!process_alive(child_pid), "child cargo still running");
    // The grandchild is orphaned when the child dies, so it leaves the process
    // table only once whoever adopted it has reaped it. A zombie answers
    // `kill(pid, 0)`, so wait for it to go instead of looking once.
    assert!(
        wait_for_gone(grandchild_pid, Duration::from_secs(5)),
        "the grandchild of a captured child should have died with its group"
    );
    reap_child.0 = None;
    reap_grandchild.0 = None;
}

/// Write an executable shell script and return its path.
#[cfg(unix)]
fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{body}")).expect("write script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("make script executable");
    path
}

/// Read a number a script wrote out.
#[cfg(unix)]
fn read_number<T: std::str::FromStr>(path: &Path) -> T
where
    T::Err: std::fmt::Debug,
{
    std::fs::read_to_string(path)
        .expect("read file")
        .trim()
        .parse()
        .expect("parse number")
}

/// Kills `pid` on drop so a stalled `sleep` cannot outlive a failed test.
#[cfg(unix)]
struct KillOnDrop(Option<libc::pid_t>);

#[cfg(unix)]
impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(pid) = self.0.take() {
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }
}

#[cfg(unix)]
fn wait_for_exit(child: &mut std::process::Child, timeout: std::time::Duration) -> ExitStatus {
    use std::time::Instant;

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("wait for cargo-q") {
            return status;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("cargo-q did not exit within {timeout:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

#[cfg(unix)]
fn process_alive(pid: libc::pid_t) -> bool {
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return true;
    }
    // EPERM means the process exists and is owned by someone else.
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Wait for a process to leave the process table.
///
/// A process that is gone but not yet reaped is a zombie, and a zombie still
/// answers `kill(pid, 0)`, so the wait has to outlast the reaping.
#[cfg(unix)]
fn wait_for_gone(pid: libc::pid_t, timeout: std::time::Duration) -> bool {
    use std::time::Instant;

    let started = Instant::now();
    loop {
        if !process_alive(pid) {
            return true;
        }
        if started.elapsed() > timeout {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
