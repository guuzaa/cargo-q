# cargo-q

A Cargo subcommand that allows running multiple Cargo commands in a time.

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

#### Sequential Execution (Space Separator)
```bash
# Run commands sequentially
cargo q check test      # Runs check, then test
cargo q "check test"    # Same as above
cargo q 'check test'    # Single and double quotes both work
```

### Commands with Arguments

```bash
# Commands with arguments need to be quoted
cargo q "test --features feature1 run"  # Run test with features, then run
```

### Parallel Execution

```bash
# Run commands in parallel
cargo q -p "build -r build"      # Run both commands in parallel
cargo q --parallel "check test"   # Same as above
```

### Verbose Output

```bash
cargo q -v "check test"       # Show detailed output
cargo q --verbose "check test"  # Same as above
```

## License

Licensed under Apache-2.0 license ([LICENSE](LICENSE) or http://opensource.org/licenses/Apache-2.0)
