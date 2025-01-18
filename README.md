# cargo-q

Cargo subcommand to run multiple Cargo commands in a time.

<details>
<summary>TODO</summary>

- ✅ Add sequential execution
- ❌ Add ; as command separator
- ❌ Add & as command separator
- ❌ Add > as command separator
- ❌ Add parallel execution

</details>

## Usage

### Run a command

```bash
cargo q cmd
```

### Run multiple commands

```bash
# default quiet mode
cargo q "check test" # run `check` first then test whether `check` is successful
cargo q 'check test' # ' and " are the same
cargo q "test --features feature1 ; run" # if a command has dash or parameters, use ; as separator

cargo q "check & test & run" # run `check` first, then `test` if `check` is successful, and `run` if both are successful
cargo q "check&test&run" # same as above

cargo q "test > analyze" # run `test` first, then `analyze` with `test`'s output
cargo q "test>analyze" # same as above

# verbose mode
cargo q -v "check test" # run `check` first, then `test` if `check` is successful
cargo q --verbose "check test" # same as above
```

### Run commands in parallel

```bash
cargo q -p "build -r; build" # run `build -r` and `build` in parallel
cargo q --parallel "build -r; build" # same as above
```
