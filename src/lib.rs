//! Library entry point for cargo-q.
//!
//! The `cargo-q` binary (see `src/main.rs`) is a thin wrapper around this
//! crate. Splitting the logic out into a library target also lets
//! `benches/` and integration tests exercise internal pieces (process
//! spawning, execution strategies) directly.

pub mod cli;
pub mod executor;
#[cfg(not(target_arch = "wasm32"))]
#[path = "process.rs"]
pub mod process;
#[cfg(target_arch = "wasm32")]
#[path = "process_wasm.rs"]
pub mod process;
pub mod progress;
pub mod routine;
pub mod strategy;
mod subcommand;

pub use crate::subcommand::Subcommands;

/// The version of cargo-q this crate was built from.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
