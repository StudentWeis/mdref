use pathdiff::diff_paths;
use std::path::{Path, PathBuf};

use crate::{MdrefError, Result};

/// Check if a link is an external URL (http, https, ftp, mailto, etc.)
pub fn is_external_url(link: &str) -> bool {
    link.contains("://")
}

/// Strip the anchor (fragment) from a link URL.
/// For example, "file.md#section" becomes "file.md", and "#section" returns None.
pub fn strip_anchor(link: &str) -> Option<&str> {
    // Handle pure anchor links (e.g., "#section") - these are internal to the file
    if link.starts_with('#') {
        return None;
    }

    // Split on '#' and take the part before it
    link.split('#').next()
}

/// Compute the relative path from one file to another.
/// Handles the case where either path may not exist yet (e.g. during dry-run)
/// by canonicalizing parent directories when possible and falling back to raw paths.
pub fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let to_resolved = resolve_path(to)?;
    let from_parent = from
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let from_resolved = if from_parent.exists() {
        from_parent.canonicalize()?
    } else {
        resolve_parent(from_parent)?
    };
    Ok(diff_paths(to_resolved, from_resolved).unwrap_or_default())
}

/// Resolve a path to its canonical form, handling non-existent files
/// by canonicalizing the nearest existing ancestor and joining the rest.
pub fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let filename = path
        .file_name()
        .ok_or_else(|| MdrefError::Path("No file name".to_string()))?;
    let parent_resolved = if parent.exists() {
        parent.canonicalize()?
    } else {
        resolve_parent(parent)?
    };
    Ok(parent_resolved.join(filename))
}

/// Resolve a directory path by canonicalizing the nearest existing ancestor
/// and joining the remaining non-existent components.
pub fn resolve_parent(dir: &Path) -> Result<PathBuf> {
    let mut components_to_append = Vec::new();
    let mut current = dir;
    loop {
        if current.exists() {
            let mut resolved = current.canonicalize()?;
            for component in components_to_append.into_iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        if let Some(file_name) = current.file_name() {
            components_to_append.push(file_name.to_owned());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(dir.to_path_buf()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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

    // ============= strip_anchor tests =============

    #[test]
    fn test_strip_anchor_with_anchor() {
        assert_eq!(strip_anchor("file.md#section"), Some("file.md"));
        assert_eq!(strip_anchor("doc.md#details"), Some("doc.md"));
    }

    #[test]
    fn test_strip_anchor_without_anchor() {
        assert_eq!(strip_anchor("file.md"), Some("file.md"));
        assert_eq!(strip_anchor("path/to/doc.md"), Some("path/to/doc.md"));
    }

    #[test]
    fn test_strip_anchor_pure_anchor() {
        assert_eq!(strip_anchor("#section"), None);
        assert_eq!(strip_anchor("#table-of-contents"), None);
    }

    // ============= relative_path tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_same_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("to.md");
        fs::write(&from, "").unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("sub").join("from.md");
        let to = temp_dir.path().join("to.md");
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        fs::write(&from, "").unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_child_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("sub").join("to.md");
        fs::write(&from, "").unwrap();
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("sub/to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_deep_cross_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("a").join("b").join("from.md");
        let to = temp_dir.path().join("x").join("y").join("to.md");
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        fs::write(&from, "").unwrap();
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../../x/y/to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_nonexistent_target() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        fs::write(&from, "").unwrap();

        let ghost = temp_dir.path().join("ghost.md");
        let result = relative_path(&from, &ghost).unwrap();
        assert_eq!(result, PathBuf::from("ghost.md"));
    }

    // ============= resolve_path tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_path_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("file.md");
        fs::write(&file, "").unwrap();

        let result = resolve_path(&file).unwrap();
        assert!(result.ends_with("file.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_path_nonexistent_file_in_existing_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("ghost.md");

        let result = resolve_path(&file).unwrap();
        assert!(result.ends_with("ghost.md"));
    }

    // ============= resolve_parent tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_parent_existing_dir() {
        let temp_dir = TempDir::new().unwrap();

        let result = resolve_parent(temp_dir.path()).unwrap();
        assert!(result.exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_parent_nonexistent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nonexist = temp_dir.path().join("a").join("b").join("c");

        let result = resolve_parent(&nonexist).unwrap();
        // Should resolve to the temp_dir + "a/b/c"
        assert!(result.ends_with("a/b/c"));
    }
}
