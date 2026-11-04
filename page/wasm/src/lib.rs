//! A WebAssembly build of cargo-q's parser.
//!
//! The landing page loads this module and asks it what a command line
//! means: the same split a run would make, from the same code that makes
//! it. Nothing here runs commands — a browser cannot — so the module
//! answers questions and does nothing else.
//!
//! The interface is a plain C ABI, so the page needs no wasm toolchain of
//! its own: it loads the module, writes its input into a buffer the module
//! owns, calls one function, and reads the answer back out.

use cargo_q::cli::Cli;
use cargo_q::Subcommands;
use std::ffi::OsString;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The input the caller wrote, and the answer to the last call.
static INPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static RESULT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// The cargo subcommand names the caller knows about.
static NAMES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// How long the answer is.
static RESULT_LEN: AtomicUsize = AtomicUsize::new(0);

/// The names cargo-q knows without asking cargo, NUL-separated.
static WELL_KNOWN: OnceLock<String> = OnceLock::new();

/// A buffer to write the next input into.
///
/// The caller writes the bytes there, and then calls the function that
/// takes them. The buffer belongs to the module: it is reused by every
/// call, and the caller never has to give it back.
#[no_mangle]
pub extern "C" fn cq_input_buffer(len: usize) -> *mut u8 {
    let mut buffer = INPUT.lock().expect("the input buffer is not poisoned");
    buffer.clear();
    buffer.resize(len, 0);
    buffer.as_mut_ptr()
}

/// Take what is in the buffer as the list of known cargo subcommand names.
///
/// The names are NUL-separated. They stay set until the next call, and
/// every inspection uses them.
#[no_mangle]
pub extern "C" fn cq_set_names(len: usize) {
    let buffer = INPUT.lock().expect("the input buffer is not poisoned");
    let len = len.min(buffer.len());
    let names: Vec<String> = String::from_utf8_lossy(&buffer[..len])
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();

    *NAMES.lock().expect("the names are not poisoned") = names;
}

/// How long the answer to the last call is.
#[no_mangle]
pub extern "C" fn cq_result_len() -> usize {
    RESULT_LEN.load(Ordering::SeqCst)
}

/// The version of cargo-q this module was built from.
///
/// Its length is [`cq_version_len`].
#[no_mangle]
pub extern "C" fn cq_version() -> *const u8 {
    cargo_q::VERSION.as_ptr()
}

/// How long the version string is.
#[no_mangle]
pub extern "C" fn cq_version_len() -> usize {
    cargo_q::VERSION.len()
}

/// The names cargo-q knows without asking cargo, NUL-separated.
///
/// Their length is [`cq_well_known_len`].
#[no_mangle]
pub extern "C" fn cq_well_known() -> *const u8 {
    WELL_KNOWN
        .get_or_init(|| Subcommands::well_known().join("\0"))
        .as_ptr()
}

/// How long the list of names is.
#[no_mangle]
pub extern "C" fn cq_well_known_len() -> usize {
    WELL_KNOWN
        .get_or_init(|| Subcommands::well_known().join("\0"))
        .len()
}

/// What a command line means, as a JSON string.
///
/// The tokens are the whole command line as a shell would pass it, program
/// name first (`cargo`, `q`, `check`, …), each one followed by a NUL, and
/// they are read from the buffer [`cq_input_buffer`] handed out. cargo-q
/// drops the program name and the `q` itself, as a run does. The NUL ends a
/// token rather than separating two, so an empty token (`cargo q check ""`)
/// reaches the parser the way a shell would pass it. The answer is a JSON
/// object:
///
/// ```text
/// {"options":["--dry-run"],"commands":[{"name":"build","args":["-r"]}],
///  "settings":{"verbose":false,"parallel":false,"keepGoing":false,
///              "dryRun":true,"noFancy":false,"repeat":1},
///  "plan":"1 command(s), sequentially:\n  cargo build -r\n",
///  "exitCode":null,"error":null,"help":null,"version":null}
/// ```
///
/// The bytes of the answer live in the module's memory; their length is
/// [`cq_result_len`], and the next call replaces them.
#[no_mangle]
pub extern "C" fn cq_inspect(len: usize) -> *const u8 {
    let tokens = {
        let buffer = INPUT.lock().expect("the input buffer is not poisoned");
        let len = len.min(buffer.len());
        String::from_utf8_lossy(&buffer[..len])
            .split_terminator('\0')
            .map(OsString::from)
            .collect::<Vec<OsString>>()
    };

    let names = NAMES.lock().expect("the names are not poisoned").clone();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let known = Subcommands::with_names(&refs);

    let inspection = Cli::inspect_argv_with(tokens, &known);
    let json = to_json(&inspection);

    let mut result = RESULT.lock().expect("the answer buffer is not poisoned");
    result.clear();
    result.extend_from_slice(json.as_bytes());
    RESULT_LEN.store(result.len(), Ordering::SeqCst);
    result.as_ptr()
}

/// The inspection, as JSON.
fn to_json(inspection: &cargo_q::cli::Inspection) -> String {
    let mut json = String::from("{\"options\":[");
    for (index, option) in inspection.options.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push('"');
        escape(&mut json, option);
        json.push('"');
    }

    json.push_str("],\"commands\":[");
    for (index, command) in inspection.commands.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str("{\"name\":\"");
        escape(&mut json, &command.name);
        json.push_str("\",\"args\":[");
        for (index, arg) in command.args.iter().enumerate() {
            if index > 0 {
                json.push(',');
            }
            json.push('"');
            escape(&mut json, arg);
            json.push('"');
        }
        json.push_str("]}");
    }

    let settings = &inspection.settings;
    json.push_str(&format!(
        "],\"settings\":{{\"verbose\":{},\"parallel\":{},\"keepGoing\":{},\"dryRun\":{},\"noFancy\":{},\"repeat\":{}}}",
        settings.verbose,
        settings.parallel,
        settings.keep_going,
        settings.dry_run,
        settings.no_fancy,
        settings.repeat,
    ));

    json.push_str(",\"plan\":");
    string_or_null(&mut json, inspection.plan.as_deref());
    json.push_str(",\"exitCode\":");
    match inspection.exit_code {
        Some(code) => json.push_str(&code.to_string()),
        None => json.push_str("null"),
    }
    json.push_str(",\"error\":");
    string_or_null(&mut json, inspection.error.as_deref());
    json.push_str(",\"help\":");
    string_or_null(&mut json, inspection.help);
    json.push_str(",\"version\":");
    string_or_null(&mut json, inspection.version);

    json.push('}');
    json
}

/// A JSON string, or `null`.
fn string_or_null(json: &mut String, text: Option<&str>) {
    match text {
        Some(text) => {
            json.push('"');
            escape(json, text);
            json.push('"');
        }
        None => json.push_str("null"),
    }
}

/// Write a string into a JSON string, without the quotes around it.
fn escape(json: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            '\u{08}' => json.push_str("\\b"),
            '\u{0c}' => json.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                json.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => json.push(c),
        }
    }
}
