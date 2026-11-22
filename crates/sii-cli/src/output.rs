//! Console output formatting, supporting human-readable Spanish text and structured JSON.
//!
//! Enforces direct `std::io::Write` locking without `println!` / `eprintln!` macros
//! (RF-03, workspace clippy lints).

use crate::error::CliError;
use std::io::Write;

/// Helper to write human-readable messages to standard output in Chilean Spanish.
pub(crate) fn print_human(message: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{message}");
}

/// Helper to write structured JSON to standard output with English keys.
pub(crate) fn print_json<T: serde::Serialize>(value: &T) {
    let mut stdout = std::io::stdout().lock();
    if let Ok(json_str) = serde_json::to_string_pretty(value) {
        let _ = writeln!(stdout, "{json_str}");
    }
}

/// Helper to write error messages to standard error in Chilean Spanish.
pub(crate) fn print_error(error: &CliError, is_json: bool) {
    let mut stderr = std::io::stderr().lock();
    if is_json {
        let payload = serde_json::json!({
            "status": "error",
            "error": error.to_string(),
        });
        if let Ok(json_str) = serde_json::to_string(&payload) {
            let _ = writeln!(stderr, "{json_str}");
        }
    } else {
        let _ = writeln!(stderr, "{error}");
    }
}
