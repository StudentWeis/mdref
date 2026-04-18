use std::io::Write;

use indicatif::{ProgressBar, ProgressStyle};
use mdref::{Result, find_links, find_references_with_progress};

pub fn run(path: String, root_dir: Option<String>, show_progress: bool) -> Result<()> {
    let mut stdout = std::io::stdout();
    run_with_writer(path, root_dir, show_progress, &mut stdout)
}

fn run_with_writer<W: Write>(
    path: String,
    root_dir: Option<String>,
    show_progress: bool,
    writer: &mut W,
) -> Result<()> {
    let root_path = root_dir.unwrap_or_else(|| ".".to_string());

    writeln!(writer, "-------------------------------")?;

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
    if references.is_empty() {
        writeln!(writer, "No references found for {path}")?;
    } else {
        writeln!(writer, "References to {path}:")?;
        for reference in references {
            writeln!(writer, "{}", reference)?;
        }
    }

    writeln!(writer, "-------------------------------")?;

    // Find all links within the specified file.
    let links = find_links(&path)?;
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

#[cfg(test)]
mod tests {
    use mdref::test_utils::write_file;
    use tempfile::TempDir;

    use crate::commands::find::run_with_writer;

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
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Path error") || error.to_string().contains("IO error"));
    }
}
