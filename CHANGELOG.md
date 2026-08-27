# Changelog

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