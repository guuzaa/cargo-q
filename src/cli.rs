use crate::executor::{Executor, Options};
use crate::process::{self, Termination};
use crate::routine::Routine;
use lexopt::prelude::*;
use lexopt::Parser;
use std::ffi::OsString;
use std::io;

/// cargo-q's own options, and the commands to run.
///
/// The options are the ones that come before the first command; everything
/// from the first command on belongs to cargo.
#[derive(Debug, Default)]
pub struct Cli {
    /// The options that were given, in the order they were given: the
    /// usage line lists them in that order.
    flags: Vec<Flag>,

    /// The cargo commands to run, with their arguments
    commands: Vec<OsString>,
}

impl Cli {
    /// Show each command's output as it runs
    #[must_use]
    pub fn verbose(&self) -> bool {
        self.flags.contains(&Flag::Verbose)
    }

    /// Run all commands at once instead of one after another (experimental)
    #[must_use]
    pub fn parallel(&self) -> bool {
        self.flags.contains(&Flag::Parallel)
    }

    /// Keep running the remaining commands after one fails
    #[must_use]
    pub fn keep_going(&self) -> bool {
        self.flags.contains(&Flag::KeepGoing)
    }

    /// Print the commands that would run, without running them
    #[must_use]
    pub fn dry_run(&self) -> bool {
        self.flags.contains(&Flag::DryRun)
    }

    /// Print plain output, without the overprinting status display
    #[must_use]
    pub fn no_fancy(&self) -> bool {
        self.flags.contains(&Flag::NoFancy)
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
        let args: Vec<OsString> = args
            .into_iter()
            // Skip all the arguments which are "q" for cargo subcommands.
            .filter(|arg| arg != "q")
            .collect();

        // The program name.
        args.into_iter().skip(1).collect()
    }

    /// Parse the arguments that follow the program name.
    fn parse_from(args: &[OsString]) -> Result<Outcome, ParseError> {
        let mut cli = Self::default();

        for (index, arg) in args.iter().enumerate() {
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
                    // take none. Only an option cargo-q knows can get
                    // here: an unknown one ends the options before its
                    // value is reached.
                    Err(lexopt::Error::UnexpectedValue { option, value }) => {
                        return Err(ParseError::UnexpectedValue {
                            option,
                            value,
                            given: cli.flags,
                        });
                    }
                    Err(err) => return Err(ParseError::Malformed(err.to_string())),
                };

                let flag = match token {
                    Short('v') | Long("verbose") => Flag::Verbose,
                    Short('p') | Long("parallel") => Flag::Parallel,
                    Short('k') | Long("keep-going") => Flag::KeepGoing,
                    Short('n') | Long("dry-run") => Flag::DryRun,
                    Long("no-fancy") => Flag::NoFancy,
                    Short('V') | Long("version") => return Ok(Outcome::Version),
                    Short('h') => return Ok(Outcome::Help(Help::Short)),
                    Long("help") => return Ok(Outcome::Help(Help::Long)),
                    // A command, or an option cargo-q does not know: either
                    // way the commands start here. Everything from the first
                    // command on belongs to cargo, not to cargo-q.
                    Short(_) | Long(_) | Value(_) => {
                        cli.commands.extend(args[index..].iter().cloned());
                        return Ok(Outcome::Run(Self::checked(cli)?));
                    }
                };

                if cli.flags.contains(&flag) {
                    return Err(ParseError::DuplicateOption(flag));
                }
                cli.flags.push(flag);
            }
        }

        Ok(Outcome::Run(Self::checked(cli)?))
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
        };
        Executor::new(routines, options).execute()
    }
}

/// One of cargo-q's own options.
///
/// Every one of them is a flag that takes no value. The type names the
/// options, so that the parser and the flags it fills in cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    /// `--verbose`
    Verbose,
    /// `--parallel`
    Parallel,
    /// `--keep-going`
    KeepGoing,
    /// `--dry-run`
    DryRun,
    /// `--no-fancy`
    NoFancy,
}

impl Flag {
    /// The name of the option, as the usage line and the errors show it.
    fn name(self) -> &'static str {
        match self {
            Flag::Verbose => "verbose",
            Flag::Parallel => "parallel",
            Flag::KeepGoing => "keep-going",
            Flag::DryRun => "dry-run",
            Flag::NoFancy => "no-fancy",
        }
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
        /// The option, as it was written.
        option: String,
        /// The value that was given.
        value: OsString,
        /// The options given before it.
        given: Vec<Flag>,
    },
    /// The same option was given twice.
    DuplicateOption(Flag),
    /// No cargo command was given.
    NoCommand {
        /// The options that were given.
        given: Vec<Flag>,
    },
    /// The command line is not something the parser understands.
    Malformed(String),
}

/// Which help text to show.
#[derive(Debug, Clone, Copy)]
enum Help {
    /// `-h`: a summary of the options.
    Short,
    /// `--help`: the full text.
    Long,
}

impl ParseError {
    /// Print the error, then end the process the way it should end.
    fn report(self) -> ! {
        match self {
            ParseError::UnexpectedValue {
                option,
                value,
                given,
            } => {
                eprint!(
                    "{}",
                    usage_error(
                        &format!(
                            "unexpected value '{}' for '{option}' found; no more were expected",
                            value.to_string_lossy()
                        ),
                        &usage_line(&given),
                    )
                );
                std::process::exit(2);
            }
            ParseError::DuplicateOption(flag) => {
                eprint!(
                    "{}",
                    usage_error(
                        &format!(
                            "the argument '--{}' cannot be used multiple times",
                            flag.name()
                        ),
                        "[OPTIONS] <COMMANDS>...",
                    )
                );
                std::process::exit(2);
            }
            ParseError::NoCommand { given } => {
                eprint!(
                    "{}",
                    usage_error(
                        "the following required arguments were not provided:\n  <COMMANDS>...",
                        &usage_line(&given),
                    )
                );
                std::process::exit(2);
            }
            ParseError::Malformed(message) => {
                eprintln!("{message}");
                std::process::exit(2);
            }
        }
    }
}

/// Print the help text, then end the process.
fn print_help(help: Help) -> ! {
    let text = match help {
        Help::Short => HELP_SHORT,
        Help::Long => HELP_LONG,
    };
    print!("{}", help_text(text));
    std::process::exit(0);
}

/// Print the version, then end the process.
fn print_version() -> ! {
    println!("cargo-q {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
}

/// A usage error, in the shape the help text uses.
fn usage_error(message: &str, usage: &str) -> String {
    format!("error: {message}\n\nUsage: cargo-q {usage}\n\nFor more information, try '--help'.\n")
}

/// The usage line for the options that were given.
fn usage_line(given: &[Flag]) -> String {
    if given.is_empty() {
        return "<COMMANDS>...".to_string();
    }

    let flags = given
        .iter()
        .map(|flag| format!("--{}", flag.name()))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{flags} <COMMANDS>...")
}

/// The help text, with the blank lines inside a description indented the way the description itself is.
fn help_text(text: &str) -> String {
    text.replace("@\n", "          \n")
}

/// The summary `-h` shows.
const HELP_SHORT: &str = r"Run Cargo only once.

Usage: cargo-q [OPTIONS] <COMMANDS>...

Arguments:
  <COMMANDS>...  Commands to execute

Options:
  -v, --verbose     Show each command's output as it runs
  -p, --parallel    Run all commands at once instead of one after another (experimental)
  -k, --keep-going  Keep running the remaining commands after one fails
  -n, --dry-run     Print the commands that would run, without running them
      --no-fancy    Print plain output, without the overprinting status display
  -h, --help        Print help (see more with '--help')
  -V, --version     Print version
";

/// The full text `--help` shows.
///
/// A line with only an `@` is a blank line inside a description: the
/// description is indented, and so is the blank line between its paragraphs.
const HELP_LONG: &str = r#"Run Cargo only once.

Usage: cargo-q [OPTIONS] <COMMANDS>...

Arguments:
  <COMMANDS>...
          Commands to execute
@
          A token that does not start with `-` starts a new command; `-` tokens are arguments to the preceding one:
@
          check test, build -r test --no-run
@
          An argument that does not start with `-` needs quoting or `=`, even when it collides with a subcommand name (e.g. a feature called `test`):
@
          "test --features f1", build --features=f1, "build --features test"

Options:
  -v, --verbose
          Show each command's output as it runs
@
          Sequential only. Combined with `--parallel`, cargo-q warns and ignores `--verbose` so the run stays quiet.

  -p, --parallel
          Run all commands at once instead of one after another (experimental)
@
          Commands that share the target directory lock each other out, so `check`/`build`/`test` may not finish any sooner than in sequence. `--verbose` is ignored in this mode; use sequential `-v` instead.

  -k, --keep-going
          Keep running the remaining commands after one fails
@
          By default cargo-q stops at the first failure, like `&&` in a shell. In parallel mode only queued commands are skipped; one already running is left to finish.

  -n, --dry-run
          Print the commands that would run, without running them
@
          Shows how a command line was split, since a bare token can be read as a new command rather than an argument.

      --no-fancy
          Print plain output, without the overprinting status display
@
          Keeps one line per command and no ANSI color even on a terminal, for output that is captured rather than read by a person: an agent driving the build through a pseudo-terminal, a log file, a CI console.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse the arguments that follow the program name.
    fn parse_args(args: &[&str]) -> Cli {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        match Cli::parse_from(&args).expect("the arguments parse") {
            Outcome::Run(cli) => cli,
            other => panic!("the arguments ask for something else: {other:?}"),
        }
    }

    #[test]
    fn hyphen_values_are_captured() {
        let cli = parse_args(&["build", "-r", "test", "--no-run"]);
        assert_eq!(
            cli.commands,
            ["build", "-r", "test", "--no-run"].map(OsString::from)
        );
        assert!(!cli.parallel());
        assert!(!cli.verbose());

        let routines = Routine::parse_many(&cli.commands).unwrap();
        assert_eq!(routines.len(), 2);
        assert_eq!(routines[0].to_string(), "cargo build -r");
        assert_eq!(routines[1].to_string(), "cargo test --no-run");
    }

    #[test]
    fn flags_before_commands_still_work() {
        let cli = parse_args(&["-p", "-v", "build", "-r"]);
        assert!(cli.parallel());
        assert!(cli.verbose());
        assert_eq!(cli.commands, ["build", "-r"].map(OsString::from));
    }

    #[test]
    fn run_control_flags_default_to_off() {
        let cli = parse_args(&["check"]);
        assert!(!cli.keep_going(), "cargo-q stops at the first failure");
        assert!(!cli.dry_run());
    }

    #[test]
    fn run_control_flags_are_accepted() {
        let cli = parse_args(&["-k", "-n", "check", "test"]);
        assert!(cli.keep_going());
        assert!(cli.dry_run());
        assert_eq!(cli.commands, ["check", "test"].map(OsString::from));

        let cli = parse_args(&["--keep-going", "--dry-run", "check"]);
        assert!(cli.keep_going());
        assert!(cli.dry_run());
    }

    /// A flag only belongs to cargo-q before the first command; after it, the
    /// parser hands every `-` token to the preceding cargo command.
    #[test]
    fn a_flag_after_a_command_is_not_a_cargo_q_flag() {
        let cli = parse_args(&["check", "--dry-run"]);
        assert!(!cli.dry_run());
        assert_eq!(cli.commands, ["check", "--dry-run"].map(OsString::from));
    }

    #[test]
    fn no_fancy_is_accepted() {
        let cli = parse_args(&["--no-fancy", "check"]);
        assert!(cli.no_fancy());

        let cli = parse_args(&["check"]);
        assert!(!cli.no_fancy(), "the overprinting console is the default");
    }

    /// Short options can be written together: `-pn` is `-p -n`.
    #[test]
    fn short_options_can_be_clustered() {
        let cli = parse_args(&["-pn", "check"]);
        assert!(cli.parallel());
        assert!(cli.dry_run());
        assert_eq!(cli.commands, ["check"].map(OsString::from));
    }

    /// An option cargo-q does not know is a command that comes before any
    /// command: the routine parser is the one to report that.
    #[test]
    fn an_unknown_option_starts_the_commands() {
        let cli = parse_args(&["--bogus", "check"]);
        assert_eq!(cli.commands, ["--bogus", "check"].map(OsString::from));
    }

    #[test]
    fn a_duplicate_option_is_an_error() {
        let args: Vec<OsString> = ["-n", "-n", "check"].iter().map(OsString::from).collect();
        let err = Cli::parse_from(&args).unwrap_err();
        assert!(
            matches!(err, ParseError::DuplicateOption(Flag::DryRun)),
            "{err:?}"
        );
    }

    #[test]
    fn no_command_is_an_error() {
        let args: Vec<OsString> = ["-v"].iter().map(OsString::from).collect();
        let err = Cli::parse_from(&args).unwrap_err();
        assert!(matches!(err, ParseError::NoCommand { .. }), "{err:?}");
    }

    #[test]
    fn help_is_asked_for() {
        let args: Vec<OsString> = ["--help"].iter().map(OsString::from).collect();
        let outcome = Cli::parse_from(&args).expect("the arguments parse");
        assert!(matches!(outcome, Outcome::Help(Help::Long)), "{outcome:?}");

        let args: Vec<OsString> = ["-h"].iter().map(OsString::from).collect();
        let outcome = Cli::parse_from(&args).expect("the arguments parse");
        assert!(matches!(outcome, Outcome::Help(Help::Short)), "{outcome:?}");
    }

    #[test]
    fn the_version_is_asked_for() {
        let args: Vec<OsString> = ["--version"].iter().map(OsString::from).collect();
        let outcome = Cli::parse_from(&args).expect("the arguments parse");
        assert!(matches!(outcome, Outcome::Version), "{outcome:?}");

        let args: Vec<OsString> = ["-V"].iter().map(OsString::from).collect();
        let outcome = Cli::parse_from(&args).expect("the arguments parse");
        assert!(matches!(outcome, Outcome::Version), "{outcome:?}");
    }

    /// Cargo runs a subcommand as `cargo-q q ...`, so the name of the
    /// subcommand is not part of the command line the user wrote.
    #[test]
    fn the_cargo_subcommand_name_is_dropped() {
        let args = Cli::command_line(["cargo-q", "q", "build"].map(OsString::from));
        assert_eq!(args, ["build"].map(OsString::from));
    }

    /// Every "q" is dropped, not only the one that names the subcommand.
    #[test]
    fn every_q_is_dropped() {
        let args = Cli::command_line(["cargo-q", "q", "build", "q"].map(OsString::from));
        assert_eq!(args, ["build"].map(OsString::from));

        let args = Cli::command_line(["cargo-q", "build", "q"].map(OsString::from));
        assert_eq!(args, ["build"].map(OsString::from));
    }

    /// A flag takes no value: `-p=1` is a mistake, not a command.
    #[test]
    fn a_value_for_a_flag_is_an_error() {
        let args: Vec<OsString> = ["-p=1", "build"].iter().map(OsString::from).collect();
        let err = Cli::parse_from(&args).unwrap_err();
        match err {
            ParseError::UnexpectedValue { option, value, .. } => {
                assert_eq!(option, "-p");
                assert_eq!(value, OsString::from("1"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    /// Everything after `--` belongs to cargo, options included.
    #[test]
    fn everything_after_a_double_dash_belongs_to_cargo() {
        let cli = parse_args(&["--", "--dry-run", "build"]);
        assert!(!cli.dry_run());
        assert_eq!(cli.commands, ["--dry-run", "build"].map(OsString::from));
    }
}
