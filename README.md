<div align="center">

<h1>cargo-q</h1>

<p><strong>Run Cargo only once.</strong></p>

<p>
  <a href="https://crates.io/crates/cargo-q"><img src="https://img.shields.io/crates/v/cargo-q?logo=rust" alt="crates.io"></a>
  <a href="https://versionlog.com/rust/1.75/"><img src="https://img.shields.io/crates/msrv/cargo-q?label=MSRV&amp;logo=rust" alt="MSRV"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue?label=License&amp;logo=Apache" alt="License"></a>
  <a href="https://crates.io/crates/cargo-q"><img src="https://img.shields.io/crates/d/cargo-q?label=Downloads" alt="Downloads"></a>
</p>

</div>

Run multiple Cargo commands sequentially or in parallel, stopping at the first
failure and returning its exit code.

[![usage](https://asciinema.org/a/xQIwh2AUWQaCPV1b.svg)](https://asciinema.org/a/xQIwh2AUWQaCPV1b)

## Installation

```bash
cargo install cargo-q --locked
```

## Features

- Run several Cargo commands in one invocation
- Stop at the first failure and exit with that command's code
- Parallel execution (experimental)
- Repeat a whole list of commands as often as you like
- Verbose mode, plus plain output for logs and agents

## Usage

### Basics

```bash
cargo q check             # one command
cargo q check test        # check, then test
```

A token starting with `-` is an argument to the preceding command:

```bash
cargo q build -r test --no-run    # cargo build -r, then cargo test --no-run
```

After the first command, any other bare token must name a Cargo subcommand;
cargo-q errors instead of guessing. Quote a command whose argument does not
start with `-`, or attach the value with `=`:

```bash
cargo q "test --features feature1"
cargo q build --features=feature1
```

Anything else is rejected with a hint that can be pasted back:

```bash
cargo q build --features feature1
# error: 'feature1' is not a cargo subcommand
# help: quote the whole command: cargo q "build --features feature1"
# help: or attach the value: cargo q build --features=feature1
```

A known subcommand name still starts a new command, even when you meant it as
an argument. Feature names and aliases like `test` collide this way:

```bash
cargo q build --features test    # cargo build --features, then cargo test
cargo q "build --features test"  # feature named test
cargo q build --features=test    # same
```

### Dry Run

`-n`/`--dry-run` shows how a command line was split, without running anything:

```bash
cargo q -n build --features test
# 2 command(s), sequentially:
#   cargo build --features
#   cargo test
```

### Repeating a Run

`--repeat <N>` runs the whole list N times over. A round is the whole list,
in the order the commands were given:

```bash
cargo q --repeat 3 check test    # check, test, check, test, check, test
```

A failure ends the run, so the later rounds never start. `-k`/`--keep-going`
runs every round anyway, and the exit code still reports the first failure.

With `-k`, every command runs once per round even when an earlier one failed,
so a failure costs the whole list N times over:

```bash
cargo q -k --repeat 3 check test    # check fails -> test still runs 3 times
```

`--repeat` repeats the list, not the command that failed: a single command
cannot be retried on its own.

The rounds run one after another, so `--repeat` cannot be combined with
`--parallel`.

### Stopping on Failure

cargo-q stops at the first failure, like `&&`, and exits with that command's
code:

```bash
cargo q clippy test    # clippy fails -> test is skipped
```

`-k`/`--keep-going` runs the rest anyway; the exit code still reports the first
failure:

```bash
cargo q -k fmt clippy test
```

In parallel mode, only queued commands are skipped; one already running is left
to finish.

### Parallel Execution (Experimental)

> [!WARNING]
> Parallel execution may not be faster. `cargo check`, `build`, and `test` share the target directory and lock it, so they block each other instead of overlapping.

```bash
cargo q -p check test
cargo q --parallel check test    # same
```

### Verbose and Plain Output

`-v`/`--verbose` streams each command's output as it runs:

```bash
cargo q -v check test
```

It is sequential only; with `--parallel`, cargo-q warns and runs quietly.

`--no-fancy` prints one line per command and no ANSI color, even on a terminal.
Use it for output that is captured rather than read: a log file, a CI console,
or an agent driving the build through a pseudo-terminal.

## For AI Agents

<details>
<summary>Guide for agents driving cargo-q through a pipe or pseudo-terminal</summary>

In a pipe cargo-q is already quiet: one line per command, no ANSI, and
diagnostics only for failures.

```bash
cargo q fmt clippy check test
```

```text
[1/4] cargo fmt
[2/4] cargo clippy
[3/4] cargo check
[4/4] cargo test
Finished 4 command(s) in 0.23s
```

- **Use the exit code.** `0` means every command passed. Otherwise you get the
  first child's own code (`101` from `cargo test`, `128 + signal` if killed,
  `130` on Ctrl-C), and later commands are skipped as with `&&`. Do not parse
  output to detect failure.
- **Failures are compact.** Each failed command prints
  `failed: cargo clippy (exit 101)` followed by the tail of its output (about
  128 KiB). Output from successful commands is discarded.
- **Check the split when in doubt.** A bare token starts a new command. Run
  `cargo q -n ...` first, quote the whole command
  (`cargo q "build --features test"`), or attach the value
  (`cargo q build --features=test`).
- **Pipes are fine.** Use `set -o pipefail` (or your shell's equivalent),
  otherwise `cargo q ... | tail` returns `tail`'s status.
- **Stay sequential.** `--parallel` rarely helps because Cargo commands lock
  the shared target directory. Add `-v` only when a failure's tail is not
  enough.

On a pseudo-terminal, add `--no-fancy` to drop the overprinting status display
and its color.

</details>

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | all commands succeeded |
| child's code | the first command that failed (e.g. `101` from `cargo test`) |
| `128 + signal` | that command was killed by a signal |
| `130` | interrupted with Ctrl-C |

## License

Licensed under Apache-2.0 license ([LICENSE](LICENSE) or http://opensource.org/licenses/Apache-2.0)
