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
