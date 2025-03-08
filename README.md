# cargo-q

A Cargo subcommand for running multiple Cargo commands sequentially or in parallel.

## Installation

```bash
cargo install cargo-q
```

## Features

- Run multiple Cargo commands sequentially
- Commands are separated by spaces
- Support parallel execution for commands
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

```bash
# For commands with arguments
cargo q "test --no-run"   # Run test with --no-run flag
cargo q "test --features feature1"  # Use quotes for complex arguments
```

### Parallel Execution

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
