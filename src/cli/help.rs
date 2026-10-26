//! The help and version text, and the functions that print them.

/// Which help text to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Help {
    /// `-h`: a summary of the options.
    Short,
    /// `--help`: the full text.
    Long,
}

/// Print the help text, then end the process.
pub(super) fn print_help(help: Help) -> ! {
    let text = match help {
        Help::Short => HELP_SHORT,
        Help::Long => HELP_LONG,
    };
    print!("{text}");
    std::process::exit(0);
}

/// Print the version, then end the process.
pub(super) fn print_version() -> ! {
    println!("cargo-q {}", env!("CARGO_PKG_VERSION"));
    std::process::exit(0);
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
      --repeat <N>  Run the whole list of commands this many times
  -h, --help        Print help (see more with '--help')
  -V, --version     Print version
";

/// The full text `--help` shows.
const HELP_LONG: &str = r#"Run Cargo only once.

Usage: cargo-q [OPTIONS] <COMMANDS>...

Arguments:
  <COMMANDS>...
          Commands to execute

          A token that does not start with `-` starts a new command; `-` tokens are arguments to the preceding one:

          check test, build -r test --no-run

          An argument that does not start with `-` needs quoting or `=`, even when it collides with a subcommand name (e.g. a feature called `test`):

          "test --features f1", build --features=f1, "build --features test"

Options:
  -v, --verbose
          Show each command's output as it runs

          Sequential only. Combined with `--parallel`, cargo-q warns and ignores `--verbose` so the run stays quiet.

  -p, --parallel
          Run all commands at once instead of one after another (experimental)

          Commands that share the target directory lock each other out, so `check`/`build`/`test` may not finish any sooner than in sequence. `--verbose` is ignored in this mode; use sequential `-v` instead.

  -k, --keep-going
          Keep running the remaining commands after one fails

          By default cargo-q stops at the first failure, like `&&` in a shell. In parallel mode only queued commands are skipped; one already running is left to finish.

          With `--repeat`, the later rounds run as well.

  -n, --dry-run
          Print the commands that would run, without running them

          Shows how a command line was split, since a bare token can be read as a new command rather than an argument.

      --no-fancy
          Print plain output, without the overprinting status display

          Keeps one line per command and no ANSI color even on a terminal, for output that is captured rather than read by a person: an agent driving the build through a pseudo-terminal, a log file, a CI console.

      --repeat <N>
          Run the whole list of commands this many times

          A round is the whole list, in the order the commands were given, so `--repeat 3 check test` runs `check test` three times over.

          Without it the list runs once.

          A failure ends the run, so the later rounds never start. With `--keep-going` every round runs anyway, and every command runs once per round even when an earlier one failed.

          The rounds run one after another, so this cannot be combined with `--parallel`.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
"#;
