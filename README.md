# cargo-q

A Cargo subcommand that allows running multiple Cargo commands in a time.

<details>
<summary>TODO</summary>

- ✅ Add sequential execution
- ✅ Add ; as command separator for independent commands
- ✅ Add & as command separator for dependent commands
- ✅ Add parallel execution between independent commands
- ❌ Add > as command separator for dependent commands
- ❌ Support mixed separators

</details>

## Installation

```bash
cargo install cargo-q
```

## Features

- Run multiple Cargo commands sequentially
- Use different separators for command execution:
  - Space: Run commands sequentially (independent execution)
  - `;`: Run independent commands sequentially
  - `&`: Run commands with dependencies (each command depends on previous command's success)
- Support parallel execution for independent commands
- Verbose mode for detailed output

## Usage

### Run a Single Command

```bash
cargo q check
```

### Run Multiple Commands

#### Sequential Execution (Space Separator)
```bash
# Run commands sequentially, each depending on previous command's success
cargo q "check test"      # Runs check, then test if check succeeds
cargo q 'check test'      # Single and double quotes both work
```

#### Independent Commands (`;` Separator)
```bash
# Run commands sequentially but independently
cargo q "test --features feature1 ; run"  # Commands with parameters need ; separator
```

#### Dependent Commands (`&` Separator)
```bash
# Run commands with explicit dependencies
cargo q "check & test & run"  # Each command runs only if previous command succeeds
cargo q "check&test&run"      # Spaces around & are optional
```

### Parallel Execution

```bash
# Run independent commands in parallel
cargo q -p "build -r; build"      # Run both commands in parallel
cargo q --parallel "check; test"   # Same as above
```

### Verbose Output

```bash
cargo q -v "check test"       # Show detailed output
cargo q --verbose "check test"  # Same as above
```

## License

Licensed under Apache-2.0 license ([LICENSE](LICENSE) or http://opensource.org/licenses/Apache-2.0)
