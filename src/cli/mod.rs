//! cargo-q's own command line.
//!
//! The options are the ones that come before the first command; everything
//! from the first command on belongs to cargo.

mod help;
#[cfg(test)]
mod tests;

use self::help::{print_help, print_version, Help};
use crate::executor::{Executor, Options};
use crate::process::{self, Termination};
use crate::routine::Routine;
use lexopt::prelude::*;
use lexopt::{Arg, Parser};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::io;

/// cargo-q's own options, and the commands to run.
#[derive(Debug, Default)]
pub struct Cli {
    /// The options that were given, in the order they were given: the
    /// usage line lists them in that order.
    flags: Vec<Flag>,

    /// How many rounds `--repeat` asked for.
    repeat: Option<u32>,

    /// The cargo commands to run, with their arguments.
    commands: Vec<OsString>,
}

impl Cli {
    /// Show each command's output as it runs.
    #[must_use]
    pub fn verbose(&self) -> bool {
        self.flags.contains(&Flag::Verbose)
    }

    /// Run all commands at once instead of one after another (experimental).
    #[must_use]
    pub fn parallel(&self) -> bool {
        self.flags.contains(&Flag::Parallel)
    }

    /// Keep running the remaining commands after one fails.
    #[must_use]
    pub fn keep_going(&self) -> bool {
        self.flags.contains(&Flag::KeepGoing)
    }

    /// Print the commands that would run, without running them.
    #[must_use]
    pub fn dry_run(&self) -> bool {
        self.flags.contains(&Flag::DryRun)
    }

    /// Print plain output, without the overprinting status display.
    #[must_use]
    pub fn no_fancy(&self) -> bool {
        self.flags.contains(&Flag::NoFancy)
    }

    /// How many times the whole list runs.
    ///
    /// `None` means the list runs once, which is what every run does
    /// without `--repeat`.
    #[must_use]
    pub fn repeat(&self) -> Option<u32> {
        self.repeat
    }

    /// Parse the command line, ending the process when it does not ask to
    /// run.
    #[must_use]
    pub fn parse() -> Self {
        let args = Self::command_line(std::env::args_os());

        match Self::parse_from(&args) {
            Ok(Outcome::Run(cli)) => cli,
            Ok(Outcome::Help(help)) => print_help(help),
            Ok(Outcome::Version) => print_version(),
            Err(err) => err.report(),
        }
    }

    /// The command line without the program name and without the "q" that
    /// names the cargo subcommand.
    ///
    /// Cargo runs a subcommand as `cargo-q q ...`: it passes the name of the
    /// subcommand along, so every "q" is removed from the command line.
    fn command_line(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
        args.into_iter()
            // Drop every "q" that names a cargo subcommand, then the
            // program name.
            .filter(|arg| arg != "q")
            .skip(1)
            .collect()
    }

    /// Parse the arguments that follow the program name.
    fn parse_from(args: &[OsString]) -> Result<Outcome, ParseError> {
        let mut cli = Self::default();

        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];

            if arg == "--" {
                // Everything after `--` belongs to cargo.
                cli.commands.extend(args[index + 1..].iter().cloned());
                break;
            }

            // One argument at a time, so that an argument cargo-q does not
            // know can be reported the way it was typed: `-vx` is one
            // token, and so is `--bogus=1`.
            let mut parser = Parser::from_args(std::iter::once(arg));
            loop {
                let token = match parser.next() {
                    Ok(Some(token)) => token,
                    Ok(None) => break,
                    // An option that was given a value: cargo-q's options
                    // take none, except `--repeat`, whose value is read
                    // below. Only an option cargo-q knows can get here: an
                    // unknown one ends the options before its value is
                    // reached.
                    Err(lexopt::Error::UnexpectedValue { option, value }) => {
                        return Err(ParseError::UnexpectedValue {
                            option,
                            value,
                            given: cli.flags,
                        });
                    }
                    Err(err) => return Err(ParseError::Malformed(err.to_string())),
                };

                match Flag::from_token(token) {
                    Some(Flag::Verbose) => cli.push(Flag::Verbose)?,
                    Some(Flag::Parallel) => cli.push(Flag::Parallel)?,
                    Some(Flag::KeepGoing) => cli.push(Flag::KeepGoing)?,
                    Some(Flag::DryRun) => cli.push(Flag::DryRun)?,
                    Some(Flag::NoFancy) => cli.push(Flag::NoFancy)?,
                    Some(Flag::Repeat) => {
                        let value = repeat_value(&mut parser, args, &mut index)?;
                        cli.set_repeat(value)?;
                    }
                    Some(Flag::Help(help)) => return Ok(Outcome::Help(help)),
                    Some(Flag::Version) => return Ok(Outcome::Version),
                    // A command, or an option cargo-q does not know: either
                    // way the commands start here. Everything from the first
                    // command on belongs to cargo, not to cargo-q.
                    None => {
                        cli.commands.extend(args[index..].iter().cloned());
                        return Ok(Outcome::Run(Self::checked(cli)?));
                    }
                }
            }

            index += 1;
        }

        Ok(Outcome::Run(Self::checked(cli)?))
    }

    /// Check that an option does not clash with one that was given before.
    ///
    /// `--repeat` and `--parallel` cannot be combined: the rounds run one
    /// after another, so there is nothing for `--parallel` to do.
    fn check(&self, flag: Flag) -> Result<(), ParseError> {
        if self.flags.contains(&flag) {
            return Err(ParseError::DuplicateOption { option: flag });
        }

        if let Some(other) = self.flags.iter().find(|other| other.conflicts(flag)) {
            return Err(ParseError::Conflict {
                first: *other,
                second: flag,
                given: self.flags.clone(),
            });
        }

        Ok(())
    }

    /// Record an option that was checked.
    fn record(&mut self, flag: Flag) {
        self.flags.push(flag);
    }

    /// Record an option, and check that it does not clash with one that was
    /// given before.
    fn push(&mut self, flag: Flag) -> Result<(), ParseError> {
        self.check(flag)?;
        self.record(flag);
        Ok(())
    }

    /// Record the round count `--repeat` asks for.
    fn set_repeat(&mut self, value: OsString) -> Result<(), ParseError> {
        // The option is looked up before its value is judged: an option
        // that is given twice is reported before its second value is
        // looked at.
        self.check(Flag::Repeat)?;

        let count = repeat_count(&value).map_err(|reason| ParseError::InvalidValue {
            option: Flag::Repeat,
            value,
            reason,
        })?;
        self.record(Flag::Repeat);
        self.repeat = Some(count);
        Ok(())
    }

    /// The parsed command line.
    ///
    /// There must be a command to run: the options alone are not enough.
    fn checked(cli: Self) -> Result<Self, ParseError> {
        if cli.commands.is_empty() {
            return Err(ParseError::NoCommand { given: cli.flags });
        }
        Ok(cli)
    }

    /// Parse-and-run entry point.
    ///
    /// The interrupt handler is installed before parsing, because resolving
    /// subcommand names spawns `cargo --list` too, and a signal during that
    /// window must end in a clean `Interrupted` instead of the default action.
    pub fn run(self) -> io::Result<Termination> {
        process::install_interrupt_handler();

        if process::was_interrupted() {
            return Ok(Termination::Interrupted);
        }

        let routines = match Routine::parse_many(&self.commands) {
            Ok(routines) => routines,
            // A signal that arrives while names are being resolved must not
            // surface as a misleading parse error.
            Err(_) if process::was_interrupted() => return Ok(Termination::Interrupted),
            Err(e) => return Err(io::Error::new(io::ErrorKind::InvalidInput, e)),
        };
        let options = Options {
            verbose: self.verbose(),
            parallel: self.parallel(),
            keep_going: self.keep_going(),
            dry_run: self.dry_run(),
            fancy: !self.no_fancy(),
            repeat: self.repeat.unwrap_or(1),
        };
        Executor::new(routines, options).execute()
    }
}

/// One of cargo-q's own options.
///
/// `Flag` names every option the parser knows, `--help` and `--version`
/// included, so that the parser and the helpers that need to recognize an
/// option share one list instead of each spelling the options out. Every
/// option but `--help` and `--version` is recorded in `Cli`; those two end
/// the parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    Verbose,
    Parallel,
    KeepGoing,
    DryRun,
    NoFancy,
    /// `--repeat`, whose value is kept in `Cli::repeat`.
    Repeat,
    /// `-h` or `--help`.
    Help(Help),
    /// `-V` or `--version`.
    Version,
}

impl Flag {
    /// The option a token names, if it is one of cargo-q's.
    fn from_token(token: Arg<'_>) -> Option<Self> {
        match token {
            Short('v') | Long("verbose") => Some(Flag::Verbose),
            Short('p') | Long("parallel") => Some(Flag::Parallel),
            Short('k') | Long("keep-going") => Some(Flag::KeepGoing),
            Short('n') | Long("dry-run") => Some(Flag::DryRun),
            Long("no-fancy") => Some(Flag::NoFancy),
            Long("repeat") => Some(Flag::Repeat),
            Short('h') => Some(Flag::Help(Help::Short)),
            Long("help") => Some(Flag::Help(Help::Long)),
            Short('V') | Long("version") => Some(Flag::Version),
            Short(_) | Long(_) | Value(_) => None,
        }
    }

    /// The option, as the usage line and the errors show it.
    fn usage(self) -> &'static str {
        match self {
            Flag::Verbose => "--verbose",
            Flag::Parallel => "--parallel",
            Flag::KeepGoing => "--keep-going",
            Flag::DryRun => "--dry-run",
            Flag::NoFancy => "--no-fancy",
            Flag::Repeat => "--repeat <N>",
            Flag::Help(Help::Short) => "-h",
            Flag::Help(Help::Long) => "--help",
            Flag::Version => "--version",
        }
    }

    /// Whether the two options cannot be used together.
    ///
    /// The rounds of a repeated run go one after another, so there is
    /// nothing for `--parallel` to do.
    fn conflicts(self, other: Flag) -> bool {
        matches!(
            (self, other),
            (Flag::Repeat, Flag::Parallel) | (Flag::Parallel, Flag::Repeat)
        )
    }
}

impl fmt::Display for Flag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.usage())
    }
}

/// What the command line asked cargo-q to do.
#[derive(Debug)]
enum Outcome {
    /// Run the commands.
    Run(Cli),
    /// Show the help.
    Help(Help),
    /// Show the version.
    Version,
}

/// Why the command line could not be parsed.
#[derive(Debug)]
enum ParseError {
    /// An option was given a value, but takes none.
    UnexpectedValue {
        option: String,
        value: OsString,
        given: Vec<Flag>,
    },
    /// The same option was given twice.
    DuplicateOption { option: Flag },
    /// Two options that cannot be used together were given, `first` before
    /// `second`.
    Conflict {
        first: Flag,
        second: Flag,
        given: Vec<Flag>,
    },
    /// An option that takes a value was given none.
    MissingValue { option: Flag },
    /// The value of an option is not one it accepts.
    InvalidValue {
        option: Flag,
        value: OsString,
        reason: String,
    },
    /// No cargo command was given.
    NoCommand { given: Vec<Flag> },
    /// The command line is not something the parser understands.
    Malformed(String),
}

impl ParseError {
    /// Print the error, then end the process the way it should end.
    fn report(self) -> ! {
        eprintln!("{}", self.message());
        std::process::exit(2);
    }

    /// The whole message: one line of explanation, then the usage line it
    /// belongs to, if there is one.
    ///
    /// A value that is missing or not accepted has no usage to show: there
    /// is no usage that would have worked.
    fn message(&self) -> String {
        let (message, usage) = match self {
            ParseError::UnexpectedValue {
                option,
                value,
                given,
            } => (
                format!(
                    "unexpected value '{}' for '{option}' found; no more were expected",
                    value.to_string_lossy()
                ),
                Some(usage_line(given)),
            ),
            ParseError::DuplicateOption { option } => (
                format!("the argument '{option}' cannot be used multiple times"),
                Some("[OPTIONS] <COMMANDS>...".to_string()),
            ),
            ParseError::Conflict {
                first,
                second,
                given,
            } => (
                format!("the argument '{first}' cannot be used with '{second}'"),
                Some(usage_line(given)),
            ),
            ParseError::MissingValue { option } => (
                format!("a value is required for '{option}' but none was supplied"),
                None,
            ),
            ParseError::InvalidValue {
                option,
                value,
                reason,
            } => (
                format!(
                    "invalid value '{}' for '{option}': {reason}",
                    value.to_string_lossy()
                ),
                None,
            ),
            ParseError::NoCommand { given } => (
                "the following required arguments were not provided:\n  <COMMANDS>...".to_string(),
                Some(usage_line(given)),
            ),
            // lexopt's own message is already a sentence: no `error:` prefix
            // and no usage line.
            ParseError::Malformed(message) => return message.clone(),
        };

        let mut text = format!("error: {message}\n\n");
        if let Some(usage) = usage {
            text.push_str(&format!("Usage: cargo-q {usage}\n\n"));
        }
        text.push_str("For more information, try '--help'.");
        text
    }
}

/// The usage line for the options that were given.
fn usage_line(given: &[Flag]) -> String {
    let flags = given
        .iter()
        .map(|flag| flag.usage())
        .collect::<Vec<_>>()
        .join(" ");
    if flags.is_empty() {
        return "<COMMANDS>...".to_string();
    }
    format!("{flags} <COMMANDS>...")
}

/// The number of rounds to run.
///
/// Zero rounds would run nothing at all, so it is rejected rather than
/// quietly turning into a successful no-op. The count is a `u32`: a run
/// that repeats the list billions of times over is a run that no machine
/// sits through, and the type keeps the arithmetic that follows it small.
fn repeat_count(value: &OsStr) -> Result<u32, String> {
    let text = value.to_string_lossy();
    let count: u64 = text
        .parse()
        .map_err(|_| format!("`{text}` is not a number of rounds"))?;
    if count == 0 {
        return Err("a run needs at least one round".to_string());
    }
    u32::try_from(count).map_err(|_| format!("`{text}` is more rounds than a run can count"))
}

/// The value of `--repeat`, written either with `=` or as the following
/// argument.
fn repeat_value(
    parser: &mut Parser,
    args: &[OsString],
    index: &mut usize,
) -> Result<OsString, ParseError> {
    match parser.value() {
        Ok(value) => Ok(value),
        Err(_) => match args.get(*index + 1) {
            Some(next) if is_repeat_value(next) => {
                *index += 1;
                Ok(next.clone())
            }
            _ => Err(ParseError::MissingValue {
                option: Flag::Repeat,
            }),
        },
    }
}

/// Whether the argument can be the value of `--repeat`.
///
/// The escape and the options cargo-q knows are not values: `--repeat
/// --dry-run` asks for a round count that is not there. An option cargo-q
/// does not know is a value, the way the arguments after the first command
/// are: `--repeat --bogus` counts the rounds of a command called
/// `--bogus`.
fn is_repeat_value(arg: &OsStr) -> bool {
    if arg == "--" {
        return false;
    }

    let mut parser = Parser::from_args(std::iter::once(arg.to_os_string()));
    match parser.next() {
        Ok(Some(token)) => Flag::from_token(token).is_none(),
        _ => true,
    }
}
