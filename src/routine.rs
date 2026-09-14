use crate::process::{self, Termination};
use crate::subcommand::Subcommands;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;

#[derive(Debug, Default, Clone)]
pub struct Routine {
    bin: OsString,
    name: OsString,
    args: Vec<OsString>,
}

impl fmt::Display for Routine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cargo {}", self.name.to_string_lossy())?;
        for arg in &self.args {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

impl Routine {
    fn new(name: OsString, args: Vec<OsString>) -> Self {
        Self {
            bin: process::cargo_bin(),
            name,
            args,
        }
    }

    /// Parse one or more routines from command-line tokens.
    ///
    /// A token that does not start with `-` starts a new command. Subsequent
    /// tokens that start with `-` are arguments to that command.
    ///
    /// A bare token is only accepted as a new command when cargo knows a
    /// subcommand by that name. Anything else is most likely an argument the
    /// parser cannot tell apart from a command, so it is rejected with a hint
    /// rather than silently running the wrong command. When cargo cannot be
    /// consulted the token is passed through instead. The first command is
    /// exempt: there it is unambiguous, and cargo reports a bad name itself.
    /// A token that matches a known subcommand still starts a new command, so
    /// an argument that collides with a name like `test` must be quoted or
    /// attached with `=`.
    ///
    /// A token that contains whitespace is split into words. If the first word
    /// starts with `-`, every word is an argument to the preceding command
    /// (e.g. `cargo q build "--features test"`). Otherwise the token is a
    /// complete command, matching quoted CLI arguments such as
    /// `"test --features f1"`.
    pub fn parse_many<I, S>(tokens: I) -> Result<Vec<Self>, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Self::parse_many_with(tokens, &Subcommands::new())
    }

    /// [`Self::parse_many`] with an explicit subcommand lookup.
    pub(crate) fn parse_many_with<I, S>(tokens: I, known: &Subcommands) -> Result<Vec<Self>, String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut routines: Vec<Routine> = vec![];

        for token in tokens {
            let token = token.as_ref();
            let (name, args) = match token.to_str() {
                Some(s) => {
                    let mut parts = s.split_whitespace();
                    let Some(name) = parts.next() else {
                        return Err("command must not be empty".to_string());
                    };
                    (OsString::from(name), parts.map(OsString::from).collect())
                }
                None => (token.to_os_string(), Vec::new()),
            };

            if name.as_encoded_bytes().starts_with(b"-") {
                match routines.last_mut() {
                    Some(routine) => {
                        routine.args.push(name);
                        routine.args.extend(args);
                    }
                    None => {
                        return Err(match name.to_str() {
                            Some(s) => {
                                format!("unexpected argument '{s}': a command must come first")
                            }
                            None => "unexpected non-UTF-8 argument: a command must come first"
                                .to_string(),
                        });
                    }
                }
            } else {
                Self::check_subcommand(&name, &args, &routines, known)?;
                routines.push(Self::new(name, args));
            }
        }

        if routines.is_empty() {
            return Err("command must not be empty".to_string());
        }

        Ok(routines)
    }

    /// Reject a bare token that would start a new command but names no cargo
    /// subcommand.
    ///
    /// Only a definite answer rejects the token: when cargo could not be
    /// consulted (`None`) the token is passed through and cargo reports it
    /// itself. The first command is exempt: there the token is unambiguous,
    /// and cargo gives a better error for a bad name anyway.
    fn check_subcommand(
        name: &OsStr,
        extra: &[OsString],
        routines: &[Self],
        known: &Subcommands,
    ) -> Result<(), String> {
        if !matches!(known.is_known(name), Some(false)) {
            return Ok(());
        }
        let Some((last, earlier)) = routines.split_last() else {
            return Ok(());
        };

        let name = name.to_string_lossy();
        let prefix = Self::q_prefix(earlier);

        let mut words = vec![last.to_q_tokens(), name.to_string()];
        words.extend(extra.iter().map(|arg| arg.to_string_lossy().into_owned()));
        let quoted_cmd = words.join(" ");

        let attached_hint = match last.attached_form(&name, extra) {
            Some(attached) => format!("\nhelp: or attach the value: {prefix}{attached}"),
            None => String::new(),
        };

        Err(format!(
            "error: '{name}' is not a cargo subcommand\n\
             help: quote the whole command: {prefix}\"{quoted_cmd}\"{attached_hint}"
        ))
    }

    /// `cargo q ` followed by already-parsed commands, each ending in a space.
    fn q_prefix(routines: &[Self]) -> String {
        let commands: String = routines.iter().fold(String::new(), |mut acc, routine| {
            acc.push_str(&routine.to_q_tokens());
            acc.push(' ');
            acc
        });
        format!("cargo q {commands}")
    }

    /// This command as cargo-q would accept it: `name` plus its arguments.
    fn to_q_tokens(&self) -> String {
        join_lossy(std::iter::once(&self.name).chain(self.args.iter()))
    }

    /// Attach `value` (and any extra words) to the last flag with `=`.
    ///
    /// Only offered when that last argument looks like a flag that still
    /// needs a value: it starts with `-` and does not already contain `=`.
    fn attached_form(&self, value: &str, extra: &[OsString]) -> Option<String> {
        let (flag, head) = self.args.split_last()?;
        let flag = flag.to_str()?;
        if !flag.starts_with('-') || flag.contains('=') {
            return None;
        }

        // Extra words are quoted with the value so they stay in one token.
        let value = match extra {
            [] => value.to_string(),
            extra => format!("\"{value} {}\"", join_lossy(extra)),
        };

        Some(format!(
            "{} {flag}={value}",
            join_lossy(std::iter::once(&self.name).chain(head))
        ))
    }

    /// Run this routine. The executable was resolved once, when the
    /// routine was parsed, so execution strategies never need to know
    /// anything about which binary is being invoked.
    pub fn run(&self, verbose: bool, output_cb: impl FnMut(&[u8])) -> io::Result<Termination> {
        process::run_command(
            &self.bin,
            std::iter::once(self.name.as_os_str()).chain(self.args.iter().map(OsString::as_os_str)),
            verbose,
            output_cb,
        )
    }
}

/// `parts` joined by spaces, each rendered as lossy UTF-8: these strings are
/// only ever shown to a human, so an argument cargo-q cannot round-trip
/// verbatim is better echoed back with replacement characters than dropped.
fn join_lossy<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    parts
        .into_iter()
        .map(|part| part.as_ref().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_many_bare_commands() {
        let routines = Routine::parse_many(["check", "test", "run"]).unwrap();
        assert_eq!(routines.len(), 3);
        assert_eq!(routines[0].name, "check");
        assert!(routines[0].args.is_empty());
        assert_eq!(routines[1].name, "test");
        assert!(routines[1].args.is_empty());
        assert_eq!(routines[2].name, "run");
        assert!(routines[2].args.is_empty());
    }

    #[test]
    fn test_parse_many_flag_args() {
        let routines = Routine::parse_many(["build", "-r", "test", "--no-run"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "build");
        assert_eq!(routines[0].args, vec!["-r"]);
        assert_eq!(routines[1].name, "test");
        assert_eq!(routines[1].args, vec!["--no-run"]);
    }

    #[test]
    fn test_parse_many_multiple_flags() {
        let routines = Routine::parse_many(["build", "-r", "--offline", "test"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "build");
        assert_eq!(routines[0].args, vec!["-r", "--offline"]);
        assert_eq!(routines[1].name, "test");
        assert!(routines[1].args.is_empty());
    }

    #[test]
    fn test_parse_many_quoted_keeps_non_flag_args() {
        let routines = Routine::parse_many(["test --features feature1", "build", "-r"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "test");
        assert_eq!(routines[0].args, vec!["--features", "feature1"]);
        assert_eq!(routines[1].name, "build");
        assert_eq!(routines[1].args, vec!["-r"]);
    }

    #[test]
    fn test_parse_many_leading_flag_is_error() {
        let err = Routine::parse_many(["-r", "test"]).unwrap_err();
        assert!(err.contains("-r"));
        let err = Routine::parse_many(["-r --offline", "test"]).unwrap_err();
        assert!(err.contains("-r"), "{err}");
    }

    #[test]
    fn quoted_flags_attach_to_the_previous_command() {
        let routines = Routine::parse_many(["build", "-r --offline", "test"]).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].name, "build");
        assert_eq!(routines[0].args, vec!["-r", "--offline"]);
        assert_eq!(routines[1].name, "test");
        assert!(routines[1].args.is_empty());
    }

    #[test]
    fn unknown_bare_token_is_rejected_with_a_hint() {
        let known = Subcommands::from_names(Some(&["check"]));
        let err = Routine::parse_many_with(["build", "--features", "f1"], &known).unwrap_err();
        assert!(err.contains("'f1'"), "{err}");
        assert!(err.contains("cargo q \"build --features f1\""), "{err}");
        assert!(err.contains("cargo q build --features=f1"), "{err}");
    }

    #[test]
    fn hint_uses_the_full_invoked_command() {
        let known = Subcommands::from_names(Some(&["check", "r"]));
        let err = Routine::parse_many_with(["r", "-p", "oven"], &known).unwrap_err();
        assert_eq!(
            err,
            "error: 'oven' is not a cargo subcommand\n\
             help: quote the whole command: cargo q \"r -p oven\"\n\
             help: or attach the value: cargo q r -p=oven"
        );

        let err = Routine::parse_many_with(["check", "r", "-p", "oven"], &known).unwrap_err();
        assert!(err.contains("cargo q check \"r -p oven\""), "{err}");
        assert!(err.contains("cargo q check r -p=oven"), "{err}");

        let err = Routine::parse_many_with(["r", "-p", "oven extra"], &known).unwrap_err();
        assert!(err.contains("cargo q \"r -p oven extra\""), "{err}");
        assert!(err.contains("cargo q r -p=\"oven extra\""), "{err}");
    }

    /// `cargo q run oven`: `oven` is read as a new command, so the hint names
    /// the command it was probably meant to accompany. With no flag to attach
    /// to, only the quoted form is offered.
    #[test]
    fn hint_quotes_the_command_the_token_belongs_to() {
        let known = Subcommands::from_names(Some(&["check"]));
        let err = Routine::parse_many_with(["run", "oven"], &known).unwrap_err();
        assert_eq!(
            err,
            "error: 'oven' is not a cargo subcommand\n\
             help: quote the whole command: cargo q \"run oven\""
        );
    }

    #[test]
    fn discovered_subcommand_starts_a_new_command() {
        let known = Subcommands::from_names(Some(&["nextest"]));
        let routines = Routine::parse_many_with(["check", "nextest"], &known).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[1].name, "nextest");
    }

    #[test]
    fn first_command_is_not_validated() {
        let known = Subcommands::from_names(Some(&[]));
        let routines = Routine::parse_many_with(["totally-unknown"], &known).unwrap();
        assert_eq!(routines.len(), 1);
        assert_eq!(routines[0].name, "totally-unknown");
    }

    #[test]
    fn undetermined_lookup_passes_the_token_through() {
        let known = Subcommands::from_names(None);
        let routines = Routine::parse_many_with(["build", "some-third-party"], &known).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[1].name, "some-third-party");
    }

    #[test]
    fn quoted_command_name_is_validated() {
        let known = Subcommands::from_names(Some(&["check"]));
        let err = Routine::parse_many_with(["check", "nope -r"], &known).unwrap_err();
        assert!(err.contains("'nope'"), "{err}");

        let routines = Routine::parse_many_with(["check", "test -r"], &known).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[1].name, "test");
        assert_eq!(routines[1].args, vec!["-r"]);
    }

    #[test]
    fn test_parse_many_empty_is_error() {
        Routine::parse_many(Vec::<&str>::new()).unwrap_err();
        Routine::parse_many([""]).unwrap_err();
        Routine::parse_many(["   "]).unwrap_err();
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_arguments() {
        use std::os::unix::ffi::OsStringExt;

        let arg = OsString::from_vec(vec![b'-', 0xff]);
        let routines = Routine::parse_many([OsString::from("test"), arg.clone()]).unwrap();
        assert_eq!(routines[0].args, vec![arg]);
    }
}
