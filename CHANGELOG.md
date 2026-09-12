# Changelog

## [0.4.0] - 2026-09-12
### Added
- `-n/--dry-run` prints the commands that would run, one per line, and runs nothing. Splitting a command line into commands involves guesswork cargo-q cannot always get right, so this is how to check what an invocation means before it spawns anything (e.g. `cargo q --dry-run build --features test` shows that `test` became a second command).
- `-k`/`--keep-going` runs the remaining commands after one fails, which was the previous behaviour.

### Changed
- **Breaking:** a failed command now stops the run, matching `&&` in a shell, instead of continuing through the rest of the list. The exit code is unchanged — still the first failure's code — and the summary counts what never started (`0 succeeded, 1 failed, 1 skipped`). Use `-k` to get the old behaviour.
- In parallel mode, a failure skips the commands still queued; commands already running are always left to finish, since killing one would leave a half-written target directory behind. With no more commands than threads, everything has already started and nothing is skipped.

## [0.3.1] - 2026-09-10
### Fixed
- Failed commands now exit with the child's status (`128 + signal` if killed by a signal) instead of always exiting 0, so `set -e`, `&&` chains, and CI can detect failures. `cargo q test` exits 101 when tests fail; Ctrl-C still exits 130.
- Sequential mode still runs the rest of the list after a failure and reports the first failure's code; parallel mode reports whichever failure is observed first.
- The interrupt handler is now installed before parsing, because resolving subcommand names spawns `cargo --list` too. A signal during that window now ends in a clean `Interrupted` (exit 130) instead of the default signal action, and can no longer be mistaken for a parse error.

### Changed
- After the first command, a bare token is only accepted as a new command when cargo knows that subcommand (built-ins, third-party `cargo-*` binaries, and aliases). Unknown tokens are rejected with a hint instead of silently running the wrong command (e.g. `cargo q build --features f1` used to run `cargo f1`, `cargo q r -p oven` use to run `cargo oven`). If cargo cannot be consulted, the token is passed through.
- A known subcommand name still starts a new command, so a positional argument that collides with a name or alias (`test`, `t`, …) needs quoting or `=`: `cargo q "build --features test"` / `cargo q build --features=test`.
- A quoted multi-word token whose first word starts with `-` is a list of arguments to the preceding command (e.g. `cargo q build "--features test"`), not a new command named after that flag.

## [0.3.0] - 2026-09-05
### Added
- Tokens starting with `-` are now treated as arguments to the preceding command, so flags can be passed without quoting (e.g. `cargo q build -r test --no-run`)

### Changed
- `Routine` is now cargo-agnostic: it takes the cargo path explicitly instead of hardcoding `cargo`
- Replaced hand-written `extern` declarations with the `windows-sys` and `libc` crates for FFI

## [0.2.3] - 2026-08-27

### Added
- Fancy progress bar supports verbose mode

### Fixed
- Redirect stdio and stderr when in non-verbose mode

## [0.2.2] - 2026-08-26

### Fixed
- Failed commands now print captured stdout as well as stderr, so `cargo test` panics and assertion details are no longer missing
- Dumb console prefixes failure output with `failed: <command>`
- Spawned commands now use the `CARGO` environment variable (falling back to `cargo` on PATH) so the same toolchain is used

### Changed
- Fancy progress is enabled when stdout is a terminal; stdin no longer needs to be a TTY
- Parallel execution caps the thread pool at the available CPU count instead of a hard limit of 8

## [0.2.1] - 2026-08-25

### Added
- Added n2-like progress display for better user feedback
- Added support for Rust 1.78 as minimum supported Rust version (MSRV)

### Fixed
- Fixed test configuration for MSRV 1.78 on macOS

## [0.2.0] - 2025-03-08

### Changed
- Simplified command parsing by removing `;` and `&` separators
- Commands are now only separated by spaces 
- For commands with arguments, you need to quote the entire command
- Updated documentation to reflect the new simplified syntax
- Removed dependent execution strategy (previously used with `&` separator)
- Improved parser to directly use parsed CLI arguments instead of re-parsing a command string

### Fixed
- Improved handling of commands with arguments

## [0.1.6] - 2025-03-07

### Added
- Initial release
- Support for running multiple commands sequentially or in parallel
- Support for different separators (space, `;`, `&`)
- Verbose mode for detailed output 