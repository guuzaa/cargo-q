use super::help::Help;
use super::*;
use crate::routine::Routine;
use std::ffi::OsString;

/// The arguments as the parser takes them.
fn os_args(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

/// What the arguments ask cargo-q to do.
fn parse_outcome(args: &[&str]) -> Outcome {
    Cli::parse_from(&os_args(args)).expect("the arguments parse")
}

/// The parsed command line.
fn parse_args(args: &[&str]) -> Cli {
    match parse_outcome(args) {
        Outcome::Run(cli) => cli,
        other => panic!("the arguments ask for something else: {other:?}"),
    }
}

/// The error the arguments are rejected with.
fn parse_error(args: &[&str]) -> ParseError {
    Cli::parse_from(&os_args(args)).expect_err("the arguments are rejected")
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
    let err = parse_error(&["-n", "-n", "check"]);
    assert!(matches!(err, ParseError::DuplicateOption { .. }), "{err:?}");
}

#[test]
fn no_command_is_an_error() {
    let err = parse_error(&["-v"]);
    assert!(matches!(err, ParseError::NoCommand { .. }), "{err:?}");
}

#[test]
fn help_is_asked_for() {
    let outcome = parse_outcome(&["--help"]);
    assert!(matches!(outcome, Outcome::Help(Help::Long)), "{outcome:?}");

    let outcome = parse_outcome(&["-h"]);
    assert!(matches!(outcome, Outcome::Help(Help::Short)), "{outcome:?}");
}

#[test]
fn the_version_is_asked_for() {
    for args in [&["--version"][..], &["-V"][..]] {
        let outcome = parse_outcome(args);
        assert!(matches!(outcome, Outcome::Version), "{outcome:?}");
    }
}

/// Cargo runs a subcommand as `cargo-q q ...`, so the name of the
/// subcommand is not part of the command line the user wrote.
#[test]
fn the_cargo_subcommand_name_is_dropped() {
    let args = Cli::command_line(os_args(&["cargo-q", "q", "build"]));
    assert_eq!(args, ["build"].map(OsString::from));
}

/// Every "q" is dropped, not only the one that names the subcommand.
#[test]
fn every_q_is_dropped() {
    let args = Cli::command_line(os_args(&["cargo-q", "q", "build", "q"]));
    assert_eq!(args, ["build"].map(OsString::from));

    let args = Cli::command_line(os_args(&["cargo-q", "build", "q"]));
    assert_eq!(args, ["build"].map(OsString::from));
}

/// A flag takes no value: `-p=1` is a mistake, not a command.
#[test]
fn a_value_for_a_flag_is_an_error() {
    match parse_error(&["-p=1", "build"]) {
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

#[test]
fn repeat_is_absent_by_default() {
    let cli = parse_args(&["check"]);
    assert_eq!(cli.repeat(), None);
}

/// The value can be written with an `=` or with a space.
#[test]
fn repeat_is_accepted_before_the_commands() {
    let cli = parse_args(&["--repeat", "3", "check", "test"]);
    assert_eq!(cli.repeat(), Some(3));
    assert_eq!(cli.commands, ["check", "test"].map(OsString::from));

    let cli = parse_args(&["--repeat=3", "check"]);
    assert_eq!(cli.repeat(), Some(3));
    assert_eq!(cli.commands, ["check"].map(OsString::from));
}

/// Zero rounds would run nothing at all, which is not what asking for
/// a repeat means.
#[test]
fn repeat_rejects_zero_rounds() {
    match parse_error(&["--repeat", "0", "check"]) {
        ParseError::InvalidValue {
            option,
            value,
            reason,
        } => {
            assert_eq!(option, REPEAT);
            assert_eq!(value, OsString::from("0"));
            assert_eq!(reason, "a run needs at least one round");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

/// A round count that is not a number is a mistake, not a command.
#[test]
fn repeat_rejects_a_value_that_is_not_a_number() {
    let err = parse_error(&["--repeat=abc", "check"]);
    assert!(
        matches!(err, ParseError::InvalidValue { option: REPEAT, .. }),
        "{err:?}"
    );
}

/// A round count that is not there is a mistake, not a command: the
/// option after it is one of cargo-q's, not the value.
#[test]
fn repeat_without_a_value_is_an_error() {
    for args in [&["--repeat"][..], &["--repeat", "--dry-run", "check"][..]] {
        let err = parse_error(args);
        assert!(
            matches!(err, ParseError::MissingValue { option: REPEAT }),
            "{err:?}"
        );
    }
}

/// An option cargo-q does not know is a value, the way the arguments
/// after the first command are.
#[test]
fn an_unknown_option_is_a_round_count() {
    match parse_error(&["--repeat", "--bogus", "check"]) {
        ParseError::InvalidValue {
            option,
            value,
            reason,
        } => {
            assert_eq!(option, REPEAT);
            assert_eq!(value, OsString::from("--bogus"));
            assert_eq!(reason, "`--bogus` is not a number of rounds");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

/// The rounds run one after another, so there is nothing for
/// `--parallel` to do.
#[test]
fn repeat_conflicts_with_parallel() {
    // The option that was given first is the one the error names first.
    let cases = [
        (
            &["--repeat", "3", "-p", "check"][..],
            REPEAT,
            Flag::Parallel.usage(),
        ),
        (
            &["-p", "--repeat", "3", "check"][..],
            Flag::Parallel.usage(),
            REPEAT,
        ),
    ];
    for (args, first, second) in cases {
        match parse_error(args) {
            ParseError::Conflict {
                first: given_first,
                second: given_second,
                ..
            } => {
                assert_eq!(given_first, first);
                assert_eq!(given_second, second);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    // The default is not a conflict: `--parallel` alone still works.
    let cli = parse_args(&["-p", "check"]);
    assert!(cli.parallel());
    assert_eq!(cli.repeat(), None);
}

/// A repeat count that is given twice is a mistake.
#[test]
fn a_repeat_given_twice_is_an_error() {
    let err = parse_error(&["--repeat", "3", "--repeat", "4", "check"]);
    assert!(
        matches!(err, ParseError::DuplicateOption { option: REPEAT }),
        "{err:?}"
    );
}

#[test]
fn a_repeat_after_a_command_is_not_a_cargo_q_flag() {
    let cli = parse_args(&["check", "--repeat", "3"]);
    assert_eq!(cli.repeat(), None);
    assert_eq!(cli.commands, ["check", "--repeat", "3"].map(OsString::from));
}
