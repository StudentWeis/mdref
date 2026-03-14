use std::fs;
use std::path::{Path, PathBuf};

use crate::{MdrefError, Reference, Result, find_links, find_references};
use pathdiff::diff_paths;

/// Move references from the old file path to the new file path within Markdown files in the specified root directory.
/// This function finds all references to the old file and updates them to point to the new file.
/// Moreover, it updates links within the moved file itself to ensure they remain valid.
pub fn mv_file<P, B, D>(raw_file_path: P, new_file_path: B, root_dir: D) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let raw_file_path = raw_file_path.as_ref();
    let new_file_path = new_file_path.as_ref();
    let root_dir = root_dir.as_ref();

    // Ensure the parent directory of the new file path exists.
    if let Some(parent) = new_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Find all references to the old file path within the specified root directory.
    let references = find_references(raw_file_path, root_dir)?;

    // Copy the original file to the new file path.
    // We copy first to avoid data loss in case of errors during reference updates.
    fs::copy(raw_file_path, new_file_path)?;

    // Update all references to point to the new file path.
    for r in references {
        update_reference(&r, new_file_path)?;
    }

    let links = find_links(new_file_path)?;
    for r in links {
        update_link(&r, raw_file_path, new_file_path)?;
    }

    // Remove the original file after updating references.
    fs::remove_file(raw_file_path)?;

    Ok(())
}

/// Check whether a link text represents an external URL (e.g. https://, http://, ftp://).
fn is_external_url(link: &str) -> bool {
    link.contains("://")
}

/// Update a link within a Markdown file to point to the new file location.
fn update_link(r: &Reference, raw_filepath: &Path, new_filepath: &Path) -> Result<()> {
    // External URLs (https://, http://, etc.) are not local file paths
    // and should not be rewritten during a file move.
    if is_external_url(&r.link_text) {
        return Ok(());
    }

    let current_link_absolute_path = raw_filepath
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?
        .join(&r.link_text)
        .canonicalize()?;
    let new_file_absolute_path = new_filepath.canonicalize()?;

    let new_link_path =
        if current_link_absolute_path.eq(&raw_filepath.canonicalize().unwrap_or_default()) {
            PathBuf::from(
                new_file_absolute_path
                    .file_name()
                    .ok_or_else(|| MdrefError::Path("No file name".to_string()))?,
            )
        } else {
            relative_path(&new_file_absolute_path, &current_link_absolute_path)?
        };
    replace_link_in_file(r, new_link_path)?;
    Ok(())
}

/// Update a reference within a Markdown file to point to the new file location.
fn update_reference(r: &Reference, new_filepath: &Path) -> Result<()> {
    let current_file_path = &r.path;
    let new_link_path = relative_path(current_file_path, new_filepath)?;
    replace_link_in_file(r, new_link_path)?;
    Ok(())
}

/// Replace the old link in the specified file with the new link path.
fn replace_link_in_file(r: &Reference, new_link_path: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&r.path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if r.line > lines.len() {
        return Err(MdrefError::InvalidLine(format!(
            "Line number {} out of range for file {}",
            r.line,
            r.path.display()
        )));
    }
    let line = &lines[r.line - 1];
    let old_pattern = format!("]({})", r.link_text);
    let new_pattern = format!("]({})", new_link_path.display());
    if line.contains(&old_pattern) {
        let new_line = line.replace(&old_pattern, &new_pattern);
        lines[r.line - 1] = new_line;
        let new_content = lines.join("\n");
        if let Err(e) = fs::write(&r.path, new_content) {
            eprintln!("Error writing file {}: {}", r.path.display(), e);
            return Err(e.into());
        }
    } else {
        eprintln!(
            "Could not find link in line {} of file {}",
            r.line,
            r.path.display()
        );
    }
    Ok(())
}

/// Compute the relative path from one file to another.
fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let to_canonical = to.canonicalize()?;
    let from_parent = from
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let from_canonical: PathBuf = from_parent.canonicalize()?;
    Ok(diff_paths(to_canonical, from_canonical).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[allow(clippy::unwrap_used)]
    fn setup_test_dir(name: &str) -> String {
        let dir = format!("test_core_mv_{}", name);
        if Path::new(&dir).exists() {
            fs::remove_dir_all(&dir).ok();
        }
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn teardown_test_dir(dir: &str) {
        if Path::new(dir).exists() {
            fs::remove_dir_all(dir).ok();
        }
    }

    #[allow(clippy::unwrap_used)]
    fn write_file(path: &str, content: &str) {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    // ============= relative_path tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_same_directory() {
        let dir = setup_test_dir("rel_same");
        let from = format!("{}/from.md", dir);
        let to = format!("{}/to.md", dir);
        write_file(&from, "");
        write_file(&to, "");

        let result = relative_path(Path::new(&from), Path::new(&to)).unwrap();
        assert_eq!(result, PathBuf::from("to.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_parent_directory() {
        let dir = setup_test_dir("rel_parent");
        let from = format!("{}/sub/from.md", dir);
        let to = format!("{}/to.md", dir);
        write_file(&from, "");
        write_file(&to, "");

        let result = relative_path(Path::new(&from), Path::new(&to)).unwrap();
        assert_eq!(result, PathBuf::from("../to.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_child_directory() {
        let dir = setup_test_dir("rel_child");
        let from = format!("{}/from.md", dir);
        let to = format!("{}/sub/to.md", dir);
        write_file(&from, "");
        write_file(&to, "");

        let result = relative_path(Path::new(&from), Path::new(&to)).unwrap();
        assert_eq!(result, PathBuf::from("sub/to.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_deep_cross_directory() {
        let dir = setup_test_dir("rel_deep");
        let from = format!("{}/a/b/from.md", dir);
        let to = format!("{}/x/y/to.md", dir);
        write_file(&from, "");
        write_file(&to, "");

        let result = relative_path(Path::new(&from), Path::new(&to)).unwrap();
        assert_eq!(result, PathBuf::from("../../x/y/to.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    fn test_relative_path_nonexistent_target() {
        let dir = setup_test_dir("rel_nonexist");
        let from = format!("{}/from.md", dir);
        write_file(&from, "");

        let result = relative_path(Path::new(&from), Path::new(&format!("{}/ghost.md", dir)));
        assert!(result.is_err());

        teardown_test_dir(&dir);
    }

    // ============= replace_link_in_file tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_replace_link_in_file_basic() {
        let dir = setup_test_dir("replace_basic");
        let file_path = format!("{}/doc.md", dir);
        write_file(&file_path, "[Link](old.md)");

        let reference = Reference::new(PathBuf::from(&file_path), 1, 1, "old.md".to_string());

        replace_link_in_file(&reference, PathBuf::from("new.md")).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](new.md)"));
        assert!(!content.contains("](old.md)"));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_replace_link_in_file_preserves_other_content() {
        let dir = setup_test_dir("replace_preserve");
        let file_path = format!("{}/doc.md", dir);
        write_file(
            &file_path,
            "# Title\n\nSome text [Link](old.md) more text.\n\nAnother paragraph.",
        );

        let reference = Reference::new(PathBuf::from(&file_path), 3, 11, "old.md".to_string());

        replace_link_in_file(&reference, PathBuf::from("new.md")).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Title"));
        assert!(content.contains("Some text [Link](new.md) more text."));
        assert!(content.contains("Another paragraph."));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_replace_link_in_file_line_out_of_range() {
        let dir = setup_test_dir("replace_oor");
        let file_path = format!("{}/doc.md", dir);
        write_file(&file_path, "Single line");

        let reference = Reference::new(PathBuf::from(&file_path), 999, 1, "link.md".to_string());

        let result = replace_link_in_file(&reference, PathBuf::from("new.md"));
        assert!(result.is_err());

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_replace_link_in_file_with_subdirectory_path() {
        let dir = setup_test_dir("replace_subdir");
        let file_path = format!("{}/doc.md", dir);
        write_file(&file_path, "[Link](sub/old.md)");

        let reference = Reference::new(PathBuf::from(&file_path), 1, 1, "sub/old.md".to_string());

        replace_link_in_file(&reference, PathBuf::from("other/new.md")).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](other/new.md)"));

        teardown_test_dir(&dir);
    }

    // ============= update_reference tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_same_directory() {
        let dir = setup_test_dir("upd_ref_same");
        let ref_file = format!("{}/ref.md", dir);
        let new_target = format!("{}/new_target.md", dir);
        write_file(&ref_file, "[Link](old_target.md)");
        write_file(&new_target, "");

        let reference = Reference::new(PathBuf::from(&ref_file), 1, 1, "old_target.md".to_string());

        update_reference(&reference, Path::new(&new_target)).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("new_target.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_cross_directory() {
        let dir = setup_test_dir("upd_ref_cross");
        let ref_file = format!("{}/ref.md", dir);
        let new_target = format!("{}/sub/new_target.md", dir);
        write_file(&ref_file, "[Link](old.md)");
        write_file(&new_target, "");

        let reference = Reference::new(PathBuf::from(&ref_file), 1, 1, "old.md".to_string());

        update_reference(&reference, Path::new(&new_target)).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("sub/new_target.md"));

        teardown_test_dir(&dir);
    }

    // ============= is_external_url tests =============

    #[test]
    fn test_is_external_url_https() {
        assert!(is_external_url("https://google.com"));
        assert!(is_external_url("https://github.com/user/repo"));
    }

    #[test]
    fn test_is_external_url_http() {
        assert!(is_external_url("http://example.com"));
        assert!(is_external_url("http://localhost:8080/path"));
    }

    #[test]
    fn test_is_external_url_other_protocols() {
        assert!(is_external_url("ftp://files.example.com/doc.md"));
        assert!(is_external_url("mailto://user@example.com"));
    }

    #[test]
    fn test_is_external_url_local_paths() {
        assert!(!is_external_url("local.md"));
        assert!(!is_external_url("sub/dir/file.md"));
        assert!(!is_external_url("../parent/file.md"));
        assert!(!is_external_url("./relative.md"));
        assert!(!is_external_url("image.png"));
    }

    // ============= update_link with external URL =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_link_skips_external_url() {
        let dir = setup_test_dir("upd_link_ext");
        let source = format!("{}/source.md", dir);
        let target = format!("{}/target.md", dir);
        write_file(&source, "[Google](https://google.com)");
        write_file(&target, "[Google](https://google.com)");

        let reference = Reference::new(
            PathBuf::from(&target),
            1,
            1,
            "https://google.com".to_string(),
        );

        // Should succeed without error — external URL is skipped
        let result = update_link(&reference, Path::new(&source), Path::new(&target));
        assert!(result.is_ok());

        // Content should remain unchanged
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("https://google.com"));

        teardown_test_dir(&dir);
    }
}
