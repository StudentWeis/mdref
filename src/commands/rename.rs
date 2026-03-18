use std::io::Write;

use mdref::{Result, rename};

pub fn run(old: String, new: String, root: Option<String>, dry_run: bool) -> Result<()> {
    let mut stdout = std::io::stdout();
    run_with_writer(old, new, root, dry_run, &mut stdout)
}

fn run_with_writer<W: Write>(
    old: String,
    new: String,
    root: Option<String>,
    dry_run: bool,
    writer: &mut W,
) -> Result<()> {
    let root_path = root.unwrap_or_else(|| ".".to_string());
    if !dry_run {
        writeln!(writer, "Rename {old} -> {new} in {root_path}")?;
    }
    rename(&old, &new, &root_path, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[allow(clippy::unwrap_used)]
    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_command_prints_summary_and_renames_file() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let source = root.join("old.md");
        write_file(&source, "# Old");

        let mut output = Vec::new();
        run_with_writer(
            source.to_str().unwrap().to_string(),
            "new.md".to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            &mut output,
        )
        .unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("Rename "));
        assert!(output.contains("old.md"));
        assert!(output.contains("new.md"));
        assert!(!source.exists());
        assert!(root.join("new.md").exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_command_dry_run_suppresses_summary_and_preserves_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let source = root.join("draft.md");
        write_file(&source, "# Draft");

        let mut output = Vec::new();
        run_with_writer(
            source.to_str().unwrap().to_string(),
            "published.md".to_string(),
            Some(root.to_str().unwrap().to_string()),
            true,
            &mut output,
        )
        .unwrap();

        assert!(String::from_utf8(output).unwrap().is_empty());
        assert!(source.exists());
        assert!(!root.join("published.md").exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_command_propagates_errors() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let mut output = Vec::new();

        let error = run_with_writer(
            root.join("missing.md").to_str().unwrap().to_string(),
            "new.md".to_string(),
            Some(root.to_str().unwrap().to_string()),
            false,
            &mut output,
        )
        .unwrap_err();

        assert!(error.to_string().contains("Source path does not exist"));
    }
}
