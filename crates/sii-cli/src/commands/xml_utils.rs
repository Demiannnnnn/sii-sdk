//! XML parsing and extraction utilities for CLI commands.

use crate::error::CliError;
use std::path::Path;

/// Finds and returns the text content within `<tag>...</tag>`.
pub(crate) fn extract_tag_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");

    let start = xml.find(&open_tag)?;
    let content_start = start.checked_add(open_tag.len())?;
    let rest = xml.get(content_start..)?;
    let end = rest.find(&close_tag)?;
    rest.get(..end)
}

/// Writes output content to a file if `output_path` is specified,
/// or to stdout if `output_path` is `None`.
pub(crate) fn write_output(content: &[u8], output_path: Option<&Path>) -> Result<(), CliError> {
    if let Some(path) = output_path {
        std::fs::write(path, content).map_err(CliError::Io)?;
    } else {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        stdout.write_all(content).map_err(CliError::Io)?;
        let _ = writeln!(stdout);
    }
    Ok(())
}
