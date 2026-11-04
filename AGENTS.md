# AGENTS.md

## What this repository is

A Cargo subcommand that runs several Cargo commands in one
invocation. The `cargo q` used below is the *installed* binary
(`~/.cargo/bin/cargo-q`), not the working tree — see "Working on cargo-q itself".

## Build, check, test

Use `cargo q` instead of `cargo` for every build, check, and test operation.
It runs several Cargo subcommands in one invocation, stops at the first
failure, and exits with that command's own code.

```bash
# type-check and test
cargo q --no-fancy check test

# release build
cargo q --no-fancy build --release

# CI gate 
cargo q --no-fancy fmt --check "clippy --all-targets -- -D warnings" check test
```

Judge the run by its exit code, never by parsing the output: the first
failure's own code reaches the shell (101 from `cargo test`, 130 on Ctrl-C).

## How `cargo q` splits the command line

- A token that starts with `-` is an argument of the command before it:
  `cargo q build -r test --no-run` is `cargo build -r`, then `cargo test --no-run`.
- Every bare token starts a new command, even when it was meant to be an
  argument: `cargo q build --features test` is `cargo build --features`, then
  `cargo test`. That split is silent, so check it when in doubt:
  `cargo q -n build --features test` prints the commands and runs nothing.
- An argument that does not start with `-` needs quoting or `=`:
  `cargo q "build --features f1"`, `cargo q build --features=f1`.
- A bare token that names no Cargo subcommand is an error:
  `error: 'feature1' is not a cargo subcommand`.
- cargo-q's own options (`-v`, `-p`, `-k`, `-n`, `--no-fancy`, `--repeat N`)
  count only before the first command. `--repeat` and `--parallel` conflict.
- At least one command is required; `cargo q` alone exits 2.

## Working on cargo-q itself

`cargo q` runs the installed `cargo-q` binary, so it does not see changes in
the working tree. When a change touches the command line (a new option, a new
splitting rule), verify it with `cargo test` — the integration test in
`tests/exit_status.rs` runs the freshly built binary through
`CARGO_BIN_EXE_cargo-q` — or run the binary in `./target` directly.

## The landing page

`page/` is the site at <https://cargo-q.paulden.site>: Astro, Tailwind, a Cloudflare
Worker. It has its own toolchain (pnpm) and its own README (`page/README.md`).

- `page/wasm/` is a small Rust crate that compiles cargo-q's parser to
  WebAssembly, so the page shows what cargo-q itself would do with a command
  line. It is a `cdylib` with a plain C ABI — no wasm-bindgen.
- The terminal at the top of the page takes input: what is typed is parsed
  by cargo-q's own parser, and the cargo commands are then *simulated*
  (`page/src/scripts/simulate.ts`), so nothing runs on the visitor's
  machine. `cargo test` fails in the simulation on purpose.
- The wasm32 build of this crate has no processes to spawn: `src/process.rs`
  is the native one, and `src/process_wasm.rs` a stub with the same API.
  `cargo q "check --target wasm32-unknown-unknown"` must keep working.
- `page/public/demo/*.ansi` are recordings of real runs; `page/README.md`
  says how to record a new one.
- The Rust CI also checks the wasm32 build; `.github/workflows/page.yml`
  deploys the site on a push to `main`.

## Project facts

- MSRV 1.75, edition 2021 (`rust-version` in Cargo.toml): no newer std APIs.
- `src/lib.rs` is the library, `src/main.rs` a thin binary.
- Tests sit next to the code in `#[cfg(test)]` modules (`src/cli/tests.rs`);
  `tests/exit_status.rs` holds the end-to-end checks. They exercise the
  public interface, not private helpers.
- `src/process_unix.rs` and `src/process_windows.rs` hold the platform
  differences (process groups, signals); CI runs Linux, macOS and Windows.
- `CHANGELOG.md` records every user-visible change, breaking library changes
  included.

## Before finishing

- `cargo q --no-fancy fmt --check "clippy --all-targets -- -D warnings" check test`
- Update `CHANGELOG.md` when the change is visible to a user.
