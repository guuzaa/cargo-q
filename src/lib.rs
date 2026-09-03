//! Library entry point for cargo-q.
//!
//! The `cargo-q` binary (see `src/main.rs`) is a thin wrapper around this
//! crate. Splitting the logic out into a library target also lets
//! `benches/` and integration tests exercise internal pieces (the thread
//! pool, process spawning, execution strategies) directly.

pub mod cli;
pub mod executor;
pub mod process;
pub mod progress;
pub mod routine;
pub mod strategy;
pub mod thread_pool;
