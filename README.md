# cargo-q
[![Crates.io MSRV](https://img.shields.io/crates/msrv/cargo-q)](https://crates.io/crates/cargo-q)
[![Apache 2.0](https://img.shields.io/badge/license-Apache-red.svg)](LICENSE)

Run multiple Cargo commands sequentially or in parallel.

[![usage](https://asciinema.org/a/xQIwh2AUWQaCPV1b.svg)](https://asciinema.org/a/xQIwh2AUWQaCPV1b)

## Installation

```bash
cargo install cargo-q --locked
```

## Features

- Run multiple Cargo commands sequentially, separated by spaces
- Stop at the first failure and exit with that command's code
- Parallel execution (experimental)
- Verbose mode

## Usage

### Run a Single Command

```bash
cargo q check
```

### Run Multiple Commands

```bash
cargo q check test      # Runs check, then test
```

### Commands with Arguments

Tokens starting with `-` are arguments to the preceding command:

```bash
cargo q build -r test --no-run
```

Quote a command whose argument does not start with `-`:

```bash
cargo q "test --features feature1"
```

Any other bare token must name a cargo subcommand; cargo-q errors instead of
guessing:

```bash
cargo q build --features feature1
# error: 'feature1' is not a cargo subcommand
# help: quote the whole command: cargo q "build --features feature1"
# help: or attach the value: cargo q build --features=feature1
```

The hints echo the command you typed, so the fix can be pasted back:

```bash
cargo q run -p oven
# error: 'oven' is not a cargo subcommand
# help: quote the whole command: cargo q "run -p oven"
# help: or attach the value: cargo q run -p=oven
```

A bare token that *is* a subcommand still starts a new command, even when you
meant it as an argument. Feature names like `test` and aliases like `t`
collide this way:

```bash
cargo q build --features test    # runs `cargo build --features` and `cargo test`
cargo q "build --features test"  # passes test as the feature
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

### Stopping on Failure

cargo-q stops at the first failure, like `&&` in a shell, and exits with that
command's own code:

```bash
cargo q clippy test    # clippy fails -> test is skipped
```

`-k`/`--keep-going` runs the whole list anyway; the exit code still reports the
first failure:

```bash
cargo q -k fmt clippy test
```

In parallel mode only queued commands are skipped — one already running is left
to finish.

### Parallel Execution (Experimental)

> [!WARNING]
> Parallel execution may not be any faster. Commands like `cargo check`, `cargo build`, and `cargo test` share the same target directory and lock it, so they block each other instead of overlapping.

```bash
cargo q -p check test            # Run both commands in parallel
cargo q --parallel check test    # Same as above
```

### Verbose Output

```bash
cargo q -v check test            # Show each command's output as it runs
cargo q --verbose check test     # Same as above
```

## Exit Codes

| Code | Meaning |
| --- | --- |
| `0` | all commands succeeded |
| child's code | the first command that failed (e.g. `101` from `cargo test`) |
| `128 + signal` | that command was killed by a signal |
| `130` | interrupted with Ctrl-C |

## License

Licensed under Apache-2.0 license ([LICENSE](LICENSE) or http://opensource.org/licenses/Apache-2.0)
