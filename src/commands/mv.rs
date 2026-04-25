use std::io::Write;

use mdref::{Result, core::mv::preview_move_with_progress, mv_with_progress};
use serde::Serialize;

use crate::commands::{
    OutputFormat, json_move_changes, progress, write_json_output, write_move_preview_human,
};

pub fn run(
    source: String,
    dest: String,
    root: Option<String>,
    dry_run: bool,
    show_progress: bool,
    format: OutputFormat,
) -> Result<()> {
    let mut stdout = std::io::stdout();
    run_with_writer(
        source,
        dest,
        root,
        dry_run,
        show_progress,
        format,
        &mut stdout,
    )
}

fn run_with_writer<W: Write>(
    source: String,
    dest: String,
    root: Option<String>,
    dry_run: bool,
    show_progress: bool,
    format: OutputFormat,
    writer: &mut W,
) -> Result<()> {
    let root = root.unwrap_or_else(|| ".".to_string());

    let progress = progress::create_spinner(show_progress && !dry_run);

    match format {
        OutputFormat::Human => {
            if dry_run {
                let preview = preview_move_with_progress(&source, &dest, &root, None)?;
                return write_move_preview_human(&preview, writer);
            }

            writeln!(writer, "Move {source} -> {dest} in {root}")?;
            let result = mv_with_progress(&source, &dest, &root, false, progress.as_ref());

            progress::finish(&progress);

            result
        }
        OutputFormat::Json => {
            let preview = preview_move_with_progress(&source, &dest, &root, None)?;

            if !dry_run {
                mv_with_progress(&source, &dest, &root, false, progress.as_ref())?;
            }

            progress::finish(&progress);

            let payload = MoveCommandOutput {
                operation: "mv",
                source,
                destination: preview.destination.display().to_string(),
                root,
                dry_run,
                changes: json_move_changes(&preview),
            };

            write_json_output(writer, &payload)
        }
    }
}

#[derive(Serialize)]
struct MoveCommandOutput {
    operation: &'static str,
    source: String,
    destination: String,
    root: String,
    dry_run: bool,
    changes: Vec<crate::commands::JsonMoveChange>,
}

#[cfg(test)]
mod tests {
    use mdref::test_utils::write_file;
    use serde_json::Value;
    use tempfile::TempDir;

    use super::*;
    use crate::commands::OutputFormat;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_mv_command_prints_summary_and_moves_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let source = root.join("source.md");
        let target = root.join("archive").join("target.md");
        write_file(&source, "# Source");

        let mut output = Vec::new();
        run_with_writer(
            source.to_str().unwrap().to_string(),
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Move "));
        assert!(output.contains("source.md"));
        assert!(output.contains("target.md"));
        assert!(!source.exists());
        assert!(target.exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_mv_command_dry_run_suppresses_summary_and_preserves_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let source = root.join("source.md");
        let target = root.join("target.md");
        write_file(&source, "# Source");

        let mut output = Vec::new();
        run_with_writer(
            source.to_str().unwrap().to_string(),
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            true,
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("[dry-run] Would move:"));
        assert!(source.exists());
        assert!(!target.exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_mv_command_propagates_errors() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let mut output = Vec::new();

        let error = run_with_writer(
            root.join("missing.md").to_str().unwrap().to_string(),
            root.join("target.md").to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            false,
            OutputFormat::Human,
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("source path does not exist"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_mv_command_writes_json_payload_for_dry_run() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let source = root.join("source.md");
        let target = root.join("archive").join("source.md");
        let reference = root.join("index.md");
        write_file(&source, "# Source");
        write_file(&reference, "See [Source](source.md)");

        let mut output = Vec::new();
        run_with_writer(
            source.to_str().unwrap().to_string(),
            target.to_str().unwrap().to_string(),
            Some(root.to_str().unwrap().to_string()),
            true,
            false,
            OutputFormat::Json,
            &mut output,
        )
        .unwrap();

        let payload: Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(payload["operation"], "mv");
        assert_eq!(payload["source"], source.to_str().unwrap());
        assert_eq!(payload["destination"], target.to_str().unwrap());
        assert_eq!(payload["dry_run"], true);
        let changes = payload["changes"].as_array().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["path"], reference.to_str().unwrap());
        assert_eq!(changes[0]["kind"], "reference_update");
        assert_eq!(changes[0]["replacements"].as_array().unwrap().len(), 1);
    }
}
