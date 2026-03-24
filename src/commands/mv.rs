use std::io::Write;

use mdref::{Result, mv};

pub fn run(source: String, dest: String, root: Option<String>, dry_run: bool) -> Result<()> {
    let mut stdout = std::io::stdout();
    run_with_writer(source, dest, root, dry_run, &mut stdout)
}

fn run_with_writer<W: Write>(
    source: String,
    dest: String,
    root: Option<String>,
    dry_run: bool,
    writer: &mut W,
) -> Result<()> {
    let root = root.unwrap_or_else(|| ".".to_string());
    if !dry_run {
        writeln!(writer, "Move {source} -> {dest} in {root}")?;
    }
    mv(&source, &dest, &root, dry_run)
}

#[cfg(test)]
mod tests {
    use mdref::test_utils::write_file;
    use tempfile::TempDir;

    use super::*;

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
            &mut output,
        )
        .unwrap();

        assert!(String::from_utf8(output).unwrap().is_empty());
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
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Source path does not exist"));
    }
}
