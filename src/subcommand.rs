//! Resolving cargo subcommand names.
//!
//! cargo-q has to decide whether a token that does not start with `-` starts a
//! new command or is an argument to the previous one. The `-` prefix makes
//! arguments unambiguous; a bare token is only accepted as a command when
//! cargo knows a subcommand by that name, so a mistyped argument fails loudly
//! instead of silently spawning the wrong cargo command.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::process::Command;
use std::sync::OnceLock;

use crate::process;

/// Cargo's built-in subcommands, plus `clippy` and `fmt`, which every Rust
/// toolchain ships as separate `cargo-*` binaries. Checking these first keeps
/// the common case spawn-free.
const WELL_KNOWN: &[&str] = &[
    "add",
    "bench",
    "build",
    "check",
    "clean",
    "clippy",
    "config",
    "doc",
    "fetch",
    "fix",
    "fmt",
    "generate-lockfile",
    "help",
    "info",
    "init",
    "install",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "new",
    "owner",
    "package",
    "pkgid",
    "publish",
    "remove",
    "report",
    "run",
    "rustc",
    "rustdoc",
    "search",
    "test",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

/// Names cargo accepts as a subcommand.
///
/// Built-in names are recognized without any work. Anything else — third-party
/// `cargo-*` binaries and user aliases from `.cargo/config.toml` — is resolved
/// once through `cargo --list`, and only when a bare token actually needs to be
/// checked.
#[derive(Debug, Default)]
pub struct Subcommands {
    discovered: OnceLock<Option<HashSet<String>>>,
}

impl Subcommands {
    pub fn new() -> Self {
        Self::default()
    }

    /// A lookup over an explicit set, for tests.
    ///
    /// `None` is a lookup that never consulted cargo.
    #[cfg(test)]
    pub fn from_names(names: Option<&[&str]>) -> Self {
        let discovered = OnceLock::new();
        let _ =
            discovered.set(names.map(|names| names.iter().copied().map(str::to_owned).collect()));
        Self { discovered }
    }

    /// Whether `name` names a cargo subcommand.
    ///
    /// `None` means cargo could not be consulted — it is missing, failed to
    /// start, or its `--list` run was killed — so no conclusion can be drawn
    /// and callers must not reject the token on the strength of a missing
    /// answer. A subcommand name is always valid UTF-8, so anything else is
    /// `Some(false)` without asking cargo.
    pub fn is_known(&self, name: &OsStr) -> Option<bool> {
        let Some(name) = name.to_str() else {
            return Some(false);
        };
        if WELL_KNOWN.contains(&name) {
            return Some(true);
        }
        self.discovered
            .get_or_init(discover)
            .as_ref()
            .map(|names| names.contains(name))
    }
}

/// Ask cargo for every subcommand it can see: built-ins, aliases, and
/// `cargo-*` binaries on `PATH`.
///
/// `None` when cargo cannot be consulted; a name must not be rejected on the
/// strength of a missing answer.
fn discover() -> Option<HashSet<String>> {
    list_from_cargo(Command::new(process::cargo_bin()))
}

fn list_from_cargo(mut cargo: Command) -> Option<HashSet<String>> {
    // Force a plain listing. `CARGO_TERM_COLOR=always` (or `term.color =
    // "always"`) colorizes `cargo --list` even when stdout is a pipe, and the
    // CSI sequences would then be parsed as part of the name.
    let output = cargo
        .args(["--color", "never", "--list"])
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.starts_with(char::is_whitespace))
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_known_names_need_no_discovery() {
        let known = Subcommands::new();
        assert_eq!(known.is_known(OsStr::new("check")), Some(true));
        assert_eq!(known.is_known(OsStr::new("clippy")), Some(true));
    }

    #[test]
    fn from_names_is_a_plain_lookup() {
        let known = Subcommands::from_names(Some(&["nextest"]));
        assert_eq!(known.is_known(OsStr::new("nextest")), Some(true));
        assert_eq!(known.is_known(OsStr::new("check")), Some(true));
        assert_eq!(known.is_known(OsStr::new("f1")), Some(false));
    }

    #[test]
    fn undetermined_lookup_answers_nothing() {
        let known = Subcommands::from_names(None);
        assert_eq!(known.is_known(OsStr::new("nextest")), None);
        assert_eq!(known.is_known(OsStr::new("f1")), None);
        // Built-ins are still recognized without cargo.
        assert_eq!(known.is_known(OsStr::new("check")), Some(true));
    }

    #[test]
    fn discovery_includes_cargo_builtins() {
        let names = discover().expect("cargo --list");
        assert!(names.contains("build"), "{names:?}");
        assert!(names.contains("test"), "{names:?}");
    }

    #[test]
    fn discovery_overrides_forced_color() {
        let mut cargo = Command::new(process::cargo_bin());
        cargo.env("CARGO_TERM_COLOR", "always");
        let names = list_from_cargo(cargo).expect("cargo --list");
        assert!(names.contains("build"), "{names:?}");
        assert!(names.contains("test"), "{names:?}");
        assert!(
            names.iter().all(|name| !name.contains('\u{1b}')),
            "{names:?}"
        );
    }

    #[test]
    fn discovery_reports_nothing_when_cargo_cannot_run() {
        assert_eq!(
            list_from_cargo(Command::new("cargo-q-no-such-binary")),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn discovery_reports_nothing_when_the_listing_fails() {
        // `sh` does not understand cargo's flags, so it exits non-zero.
        assert_eq!(list_from_cargo(Command::new("sh")), None);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_names_are_never_known() {
        use std::os::unix::ffi::OsStrExt;
        let name = OsStr::from_bytes(&[0xff]);
        assert_eq!(Subcommands::new().is_known(name), Some(false));
    }
}
