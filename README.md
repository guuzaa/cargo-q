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
