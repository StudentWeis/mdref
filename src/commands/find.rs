use std::io::Write;

use indicatif::{ProgressBar, ProgressStyle};
use mdref::{MdrefError, Reference, Result, find_links, find_references_with_progress};
use serde::Serialize;

use super::OutputFormat;

pub fn run(
    path: String,
    root_dir: Option<String>,
    show_progress: bool,
    format: OutputFormat,
) -> Result<()> {
    let mut stdout = std::io::stdout();
    run_with_writer(path, root_dir, show_progress, format, &mut stdout)
}

fn run_with_writer<W: Write>(
    path: String,
    root_dir: Option<String>,
    show_progress: bool,
    format: OutputFormat,
    writer: &mut W,
) -> Result<()> {
    let root_path = root_dir.unwrap_or_else(|| ".".to_string());

    // Find references to the specified file.
    let progress = if show_progress {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::with_template("{spinner:.green} [{pos}/{len}] {msg}")
                .expect("valid template"),
        );
        Some(progress_bar)
    } else {
        None
    };

    let references = find_references_with_progress(&path, &root_path, progress.as_ref())?;

    if let Some(progress_bar) = &progress {
        progress_bar.finish_and_clear();
    }

    // Find all links within the specified file.
    let links = find_links(&path)?;

    match format {
        OutputFormat::Human => write_human_output(&path, &references, &links, writer),
        OutputFormat::Json => write_json_output(&path, &references, &links, writer),
    }
}

fn write_human_output<W: Write>(
    path: &str,
    references: &[Reference],
    links: &[Reference],
    writer: &mut W,
) -> Result<()> {
    writeln!(writer, "-------------------------------")?;

    if references.is_empty() {
        writeln!(writer, "No references found for {path}")?;
    } else {
        writeln!(writer, "References to {path}:")?;
        for reference in references {
            writeln!(writer, "{}", reference)?;
        }
    }

    writeln!(writer, "-------------------------------")?;

    if links.is_empty() {
        writeln!(writer, "No links found in {path}")?;
    } else {
        writeln!(writer, "Links in {path}:")?;
        for link in links {
            writeln!(writer, "{}", link)?;
        }
    }

    writeln!(writer, "-------------------------------")?;

    Ok(())
}

fn write_json_output<W: Write>(
    path: &str,
    references: &[Reference],
    links: &[Reference],
    writer: &mut W,
) -> Result<()> {
    let payload = FindOutput {
        operation: "find",
        target: path,
        references: references.iter().map(JsonReference::from).collect(),
        links: links.iter().map(JsonReference::from).collect(),
    };

    serde_json::to_writer_pretty(&mut *writer, &payload)
        .map_err(|error| MdrefError::Path(format!("Failed to write JSON output: {error}")))?;
    writeln!(writer)?;

    Ok(())
}

#[derive(Serialize)]
struct FindOutput<'a> {
    operation: &'static str,
    target: &'a str,
    references: Vec<JsonReference>,
    links: Vec<JsonReference>,
}

#[derive(Serialize)]
struct JsonReference {
    path: String,
    line: usize,
    column: usize,
    link_text: String,
}

impl From<&Reference> for JsonReference {
    fn from(reference: &Reference) -> Self {
        Self {
            path: reference.path.display().to_string(),
            line: reference.line,
            column: reference.column,
            link_text: reference.link_text.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use mdref::test_utils::write_file;
    use serde_json::Value;
    use tempfile::TempDir;

    use crate::commands::{OutputFormat, find::run_with_writer};

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_find_command_reports_references_and_links() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let target = root.join("target.md");
        let index = root.join("index.md");
        write_file(&target, "[Local](guide.md)");
        write_file(&index, "See [Target](target.md)");

        let mut output = Vec::new();
        run_with_writer(
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("References to"));
        assert!(output.contains("index.md"));
        assert!(output.contains("Links in"));
        assert!(output.contains("guide.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_find_command_reports_empty_sections() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let target = root.join("target.md");
        write_file(&target, "# Target");

        let mut output = Vec::new();
        run_with_writer(
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("No references found for"));
        assert!(output.contains("No links found in"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_find_command_propagates_library_errors() {
        let mut output = Vec::new();
        let error = run_with_writer(
            "missing.md".to_string(),
            Some(".".to_string()),
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Path error") || error.to_string().contains("IO error"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_find_command_writes_json_payload() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let target = root.join("target.md");
        let index = root.join("index.md");
        write_file(&target, "[Local](guide.md)");
        write_file(&index, "See [Target](target.md)");

        let mut output = Vec::new();
        run_with_writer(
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            OutputFormat::Json,
            &mut output,
        )
        .unwrap();

        let payload: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(payload["operation"], "find");
        assert_eq!(payload["target"], target.to_str().unwrap());

        let references = payload["references"].as_array().unwrap();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0]["path"], index.to_str().unwrap());
        assert_eq!(references[0]["link_text"], "target.md");
        assert!(references[0]["line"].is_number());
        assert!(references[0]["column"].is_number());

        let links = payload["links"].as_array().unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0]["path"], target.to_str().unwrap());
        assert_eq!(links[0]["link_text"], "guide.md");
    }
}
