//! End-to-end checks that a failed command reaches the shell as a non-zero
//! exit status, carrying the child's own code. cargo-q runs commands to
//! report on them, so scripts and CI must be able to rely on the status.

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

#[test]
fn unknown_bare_token_exits_nonzero() {
    let output = cargo_q_output(&["build", "--features", "f1"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("is not a cargo subcommand"), "{stderr}");
}

/// Install a `cargo` wrapper that stalls before listing, so the window in
/// which cargo-q resolves subcommand names is wide enough to signal. It
/// records its pid and then sleeps.
#[cfg(unix)]
fn stalling_cargo(dir: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = dir.join("stalling-cargo");
    let pid_file = dir.join("stalling-cargo.pid");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho $$ > '{}'\nsleep 30\nexec cargo \"$@\"\n",
            pid_file.display()
        ),
    )
    .expect("write wrapper");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("make wrapper executable");
    pid_file
}

/// A signal during name resolution must end in a clean `Interrupted`, not the
/// default action and not a misleading "not a cargo subcommand" error.
#[cfg(unix)]
#[test]
fn interrupt_during_name_resolution_exits_interrupted() {
    use std::os::unix::process::CommandExt;
    use std::time::{Duration, Instant};

    let dir = OutsidePackage::new();
    let pid_file = stalling_cargo(&dir.0);

    let child = Command::new(env!("CARGO_BIN_EXE_cargo-q"))
        .args(["check", "some-third-party"])
        .env("CARGO", dir.0.join("stalling-cargo"))
        // Own process group, so signalling the group mimics a terminal Ctrl-C.
        .process_group(0)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("run cargo-q");

    let deadline = Instant::now() + Duration::from_secs(5);
    while !pid_file.exists() {
        assert!(Instant::now() < deadline, "cargo --list never started");
        std::thread::sleep(Duration::from_millis(10));
    }

    let killed = Command::new("kill")
        .args(["-INT", &format!("-{}", child.id())])
        .status()
        .expect("send SIGINT");
    assert!(killed.success());

    let output = child.wait_with_output().expect("wait for cargo-q");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(130), "{stderr}");
    assert!(!stderr.contains("is not a cargo subcommand"), "{stderr}");
}
