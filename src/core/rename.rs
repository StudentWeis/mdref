use std::path::Path;

use crate::{Result, mv};

/// Rename a file by changing only its filename while keeping it in the same directory.
/// This is a convenience wrapper around `mv` that handles the common case of
/// renaming a file in place.
///
/// # Arguments
///
/// * `source` - Path to the file to rename
/// * `name` - The new filename (not a path, just the filename)
/// * `root` - Root directory to search for references
/// * `dry_run` - If true, only preview changes without making them
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
///
/// # Example
///
/// ```ignore
/// use mdref::rename;
///
/// // Rename "old.md" to "new.md" in the same directory
/// rename("docs/old.md", "new.md", ".", false)?;
/// ```
pub fn rename<P, B, D>(source: P, name: B, root: D, dry_run: bool) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<str>,
    D: AsRef<Path>,
{
    let source = source.as_ref();
    let name = name.as_ref();
    let root = root.as_ref();

    // Compute the new path by replacing only the filename
    let new_path = source.with_file_name(name);

    mv(source, new_path, root, dry_run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MdrefError;
    use std::fs;
    use std::io::Write;
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
    fn test_rename_same_directory_moves_to_new_name() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("guide.md");
        write_file(&source, "# Guide");

        rename(&source, "guide-v2.md", temp_dir.path(), false).unwrap();

        assert!(!source.exists());
        assert!(temp_dir.path().join("guide-v2.md").exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_updates_external_references_in_place() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("topic.md");
        let index = temp_dir.path().join("index.md");
        write_file(&source, "# Topic");
        write_file(&index, "See [topic](topic.md).");

        rename(&source, "topic-v2.md", temp_dir.path(), false).unwrap();

        let index_content = fs::read_to_string(&index).unwrap();
        assert!(index_content.contains("topic-v2.md"));
        assert!(!index_content.contains("](topic.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_updates_self_references_in_place() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("page.md");
        write_file(&source, "[Self](page.md)\n[Section](#intro)");

        rename(&source, "page-v2.md", temp_dir.path(), false).unwrap();

        let renamed_content = fs::read_to_string(temp_dir.path().join("page-v2.md")).unwrap();
        assert!(renamed_content.contains("[Self](page-v2.md)"));
        assert!(renamed_content.contains("[Section](#intro)"));
        assert!(!renamed_content.contains("[Self](page.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_dry_run_preserves_files_and_references() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("draft.md");
        let index = temp_dir.path().join("index.md");
        write_file(&source, "# Draft\n\n[Self](draft.md)");
        write_file(&index, "[Draft](draft.md)");

        rename(&source, "published.md", temp_dir.path(), true).unwrap();

        assert!(source.exists());
        assert!(!temp_dir.path().join("published.md").exists());
        assert_eq!(
            fs::read_to_string(&source).unwrap(),
            "# Draft\n\n[Self](draft.md)"
        );
        assert_eq!(fs::read_to_string(&index).unwrap(), "[Draft](draft.md)");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_rename_propagates_mv_validation_errors() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let existing_target = temp_dir.path().join("taken.md");
        write_file(&source, "# Source");
        write_file(&existing_target, "# Existing");

        let error = rename(&source, "taken.md", temp_dir.path(), false).unwrap_err();

        assert!(matches!(error, MdrefError::Path(_)));
        assert!(
            error
                .to_string()
                .contains("Destination path already exists")
        );
        assert!(source.exists());
        assert_eq!(fs::read_to_string(&existing_target).unwrap(), "# Existing");
    }
}
