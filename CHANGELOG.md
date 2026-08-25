# Changelog

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