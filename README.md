# cargo-q

A Cargo subcommand for running multiple Cargo commands sequentially or in parallel.

[![usage](https://asciinema.org/a/YlyT7mmdtzxXI6BS.svg)](https://asciinema.org/a/YlyT7mmdtzxXI6BS)

## Installation

```bash
cargo install cargo-q --locked
```

## Features

- Run multiple Cargo commands sequentially
- Commands are separated by spaces
- Support parallel execution for commands (experimental)
- Verbose mode for detailed output

## Usage

### Run a Single Command

```bash
cargo q check
```

### Run Multiple Commands

```bash
# Run commands sequentially
cargo q check test      # Runs check, then test
```

### Commands with Arguments

Tokens that start with `-` are treated as arguments to the preceding command:

```bash
cargo q build -r test --no-run
```

Quote a command when an argument does not start with `-`:

```bash
cargo q "test --features feature1"
```

A token that does not start with `-` must name a cargo subcommand; otherwise
cargo-q reports an error instead of guessing:

```bash
cargo q build --features feature1
# error: 'feature1' is not a cargo subcommand
# help: quote the whole command to pass it as an argument: cargo q "test --features f1"
# help: or use the attached form: cargo q test --features=f1
```

A bare token that *is* a cargo subcommand still starts a new command, even
when you meant it as an argument. Feature names like `test` and default
aliases like `t` collide this way:

```bash
cargo q build --features test    # runs `cargo build --features` and `cargo test`
cargo q "build --features test"  # passes test as the feature
cargo q build --features=test    # same
```

### Parallel Execution (Experimental)

> [!WARNING]
> **Note:** Parallel execution is currently experimental and may not provide a performance improvement. Commands like `cargo check`, `cargo build`, and `cargo test` share the same target directory and lock it, so they will block each other while waiting for the lock. As a result, running these commands in parallel is not faster than running them sequentially.

```bash
# Run commands in parallel
cargo q -p check test      # Run both commands in parallel
cargo q --parallel check test   # Same as above
```

### Verbose Output

```bash
cargo q -v check test       # Show detailed output
cargo q --verbose check test  # Same as above
```

## License

Licensed under Apache-2.0 license ([LICENSE](LICENSE) or http://opensource.org/licenses/Apache-2.0)
