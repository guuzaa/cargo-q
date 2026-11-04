//! The wasm32 side of running a child: there is none.
//!
//! A browser has no processes to spawn and no signals to catch, so the
//! wasm32 build of this crate can parse a command line and say what it
//! means, but cannot run it. The API is the same one the native build has,
//! so a caller does not have to know which one it was compiled for.

use std::ffi::{OsStr, OsString};
use std::io;

/// How a run ended.
#[derive(Debug, Clone, Copy)]
pub enum Termination {
    /// Every command succeeded.
    Success,
    /// The run was interrupted.
    Interrupted,
    /// A command failed. Carries the child's exit code, or `128 + signal`
    /// when it was killed by a signal, following shell convention.
    Failure(i32),
}

impl Termination {
    /// The exit code that reports this outcome to the shell.
    #[inline]
    #[must_use]
    pub fn exit_code(self) -> i32 {
        match self {
            Termination::Success => 0,
            Termination::Interrupted => 130,
            Termination::Failure(code) => code,
        }
    }
}

/// Whether the run was interrupted.
///
/// There is nothing to interrupt in a browser, so this is always false.
#[inline]
#[must_use]
pub fn was_interrupted() -> bool {
    false
}

/// Locate the cargo binary.
///
/// There is no cargo to run in a browser, so this is the name on `PATH`.
#[inline]
#[must_use]
pub fn cargo_bin() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

/// Catch SIGINT, SIGTERM, and SIGHUP (and Windows Ctrl-C) and kill registered
/// process groups.
///
/// There are no signals in a browser, so this does nothing.
pub fn install_interrupt_handler() {}

/// Run `program` with `args`, streaming merged stdout+stderr into `output_cb`.
///
/// A browser cannot start a process, so this always fails.
pub fn run_command(
    _program: impl AsRef<OsStr>,
    _args: impl IntoIterator<Item = impl AsRef<OsStr>>,
    _verbose: bool,
    _color: bool,
    _output_cb: impl FnMut(&[u8]),
) -> io::Result<Termination> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "cargo-q cannot run commands in a browser",
    ))
}
