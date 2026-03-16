use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{MdrefError, Reference, Result, find_links, find_references};
use pathdiff::diff_paths;

/// A pending replacement: which line/column to find the old pattern, and what to replace it with.
struct LinkReplacement {
    line: usize,
    column: usize,
    old_pattern: String,
    new_pattern: String,
}

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

    // Check if source file exists before proceeding.
    if !raw_file_path.exists() {
        return Err(MdrefError::Path(format!(
            "Source file does not exist: {}",
            raw_file_path.display()
        )));
    }

    // Use canonicalize to reliably detect if source and destination point to the same file.
    // This handles cases like: "file.md" vs "./file.md", relative vs absolute paths, symlinks, etc.
    let raw_canonical = raw_file_path.canonicalize().map_err(|e| {
        MdrefError::Path(format!(
            "Cannot canonicalize source path '{}': {}",
            raw_file_path.display(),
            e
        ))
    })?;

    // For destination, canonicalize parent directory and join with filename
    // to handle cases where destination doesn't exist yet.
    let new_canonical = if new_file_path.exists() {
        new_file_path.canonicalize().map_err(|e| {
            MdrefError::Path(format!(
                "Cannot canonicalize destination path '{}': {}",
                new_file_path.display(),
                e
            ))
        })?
    } else {
        // If destination doesn't exist, resolve relative to its parent directory
        let parent = new_file_path.parent().ok_or_else(|| {
            MdrefError::Path(format!(
                "Destination path has no parent directory: {}",
                new_file_path.display()
            ))
        })?;

        // Canonicalize parent (it may or may not exist)
        let parent_canonical = if parent.exists() {
            parent.canonicalize().map_err(|e| {
                MdrefError::Path(format!(
                    "Cannot canonicalize parent directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?
        } else {
            parent.to_path_buf()
        };

        let filename = new_file_path.file_name().ok_or_else(|| {
            MdrefError::Path(format!(
                "Destination path has no filename: {}",
                new_file_path.display()
            ))
        })?;
        parent_canonical.join(filename)
    };

    // If source and destination resolve to the same file, nothing to do.
    if raw_canonical == new_canonical {
        return Ok(());
    }

    // Ensure the parent directory of the new file path exists.
    if let Some(parent) = new_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Find all references to the old file path within the specified root directory.
    let references = find_references(raw_file_path, root_dir)?;

    // Copy the original file to the new file path.
    // We copy first to avoid data loss in case of errors during reference updates.
    fs::copy(raw_file_path, new_file_path)?;

    // Collect all reference replacements, grouped by file path.
    let mut replacements_by_file: HashMap<PathBuf, Vec<LinkReplacement>> = HashMap::new();

    for reference in &references {
        // Extract the anchor from the link text (if any) to preserve it
        let (_link_path_only, anchor) = split_link_and_anchor(&reference.link_text);

        let new_link_path = relative_path(&reference.path, new_file_path)?;

        // Reconstruct the new pattern with anchor preserved
        let new_link_with_anchor = match anchor {
            Some(a) => format!("{}#{}", new_link_path.display(), a),
            None => new_link_path.display().to_string(),
        };

        let old_pattern = format!("]({})", reference.link_text);
        let new_pattern = format!("]({})", new_link_with_anchor);
        replacements_by_file
            .entry(reference.path.clone())
            .or_default()
            .push(LinkReplacement {
                line: reference.line,
                column: reference.column,
                old_pattern,
                new_pattern,
            });
    }

    // Collect all link replacements for the moved file itself.
    let links = find_links(new_file_path)?;
    for link in &links {
        if let Some(replacement) = build_link_replacement(link, raw_file_path, new_file_path)? {
            replacements_by_file
                .entry(link.path.clone())
                .or_default()
                .push(replacement);
        }
    }

    // Apply all replacements, one file at a time.
    for (file_path, replacements) in &replacements_by_file {
        apply_replacements(file_path, replacements)?;
    }

    // Remove the original file after updating references.
    fs::remove_file(raw_file_path)?;

    Ok(())
}

/// Check whether a link text represents an external URL (e.g. https://, http://, ftp://).
fn is_external_url(link: &str) -> bool {
    link.contains("://")
}

/// Split a link into the path part and the anchor (fragment) part.
/// Returns (path, Some(anchor)) if there's an anchor, or (path, None) if not.
/// Examples:
///   "file.md#section" -> ("file.md", Some("section"))
///   "file.md" -> ("file.md", None)
///   "#section" -> ("", Some("section"))  (pure anchor link)
fn split_link_and_anchor(link: &str) -> (&str, Option<&str>) {
    match link.find('#') {
        Some(pos) => {
            let (path, anchor) = link.split_at(pos);
            // Remove the '#' prefix from anchor
            (path, Some(&anchor[1..]))
        }
        None => (link, None),
    }
}

/// Build a LinkReplacement for an internal link in the moved file.
/// Returns `None` if the link is an external URL or a broken link that cannot be resolved.
fn build_link_replacement(
    r: &Reference,
    raw_filepath: &Path,
    new_filepath: &Path,
) -> Result<Option<LinkReplacement>> {
    // External URLs (https://, http://, etc.) are not local file paths
    // and should not be rewritten during a file move.
    if is_external_url(&r.link_text) {
        return Ok(None);
    }

    // Strip anchor from link text so canonicalize works on the file path only.
    let (link_path_only, anchor) = split_link_and_anchor(&r.link_text);

    // Pure anchor links (e.g. "#section") are internal to the document
    // and should not be rewritten during a file move.
    if link_path_only.is_empty() {
        return Ok(None);
    }

    let parent = raw_filepath
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;

    // Resolve the link path; skip broken links that cannot be canonicalized.
    let current_link_absolute_path = match parent.join(link_path_only).canonicalize() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    let new_file_absolute_path = new_filepath.canonicalize()?;
    let raw_file_canonical = raw_filepath.canonicalize()?;

    let new_link_path = if current_link_absolute_path == raw_file_canonical {
        PathBuf::from(
            new_file_absolute_path
                .file_name()
                .ok_or_else(|| MdrefError::Path("No file name".to_string()))?,
        )
    } else {
        relative_path(&new_file_absolute_path, &current_link_absolute_path)?
    };

    // Reconstruct the new pattern with anchor preserved.
    let new_link_with_anchor = match anchor {
        Some(a) => format!("{}#{}", new_link_path.display(), a),
        None => new_link_path.display().to_string(),
    };

    let old_pattern = format!("]({})", r.link_text);
    let new_pattern = format!("]({})", new_link_with_anchor);

    Ok(Some(LinkReplacement {
        line: r.line,
        column: r.column,
        old_pattern,
        new_pattern,
    }))
}

/// Apply all pending replacements to a single file in one read-write cycle.
/// Replacements are sorted in reverse order (by line desc, then column desc) so that
/// earlier replacements do not shift the positions of later ones.
fn apply_replacements(file_path: &Path, replacements: &[LinkReplacement]) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Sort replacements in reverse order (bottom-right to top-left) so that
    // replacing one link does not invalidate the positions of subsequent ones.
    let mut sorted_indices: Vec<usize> = (0..replacements.len()).collect();
    sorted_indices.sort_by(|&a, &b| {
        replacements[b]
            .line
            .cmp(&replacements[a].line)
            .then_with(|| replacements[b].column.cmp(&replacements[a].column))
    });

    for &idx in &sorted_indices {
        let replacement = &replacements[idx];

        if replacement.line > lines.len() {
            return Err(MdrefError::InvalidLine(format!(
                "Line number {} out of range for file {}",
                replacement.line,
                file_path.display()
            )));
        }

        let line = &lines[replacement.line - 1];
        let col = replacement.column.saturating_sub(1); // Convert to 0-based index

        // Search for the old_pattern starting from the column position.
        // This ensures we replace the correct occurrence when multiple identical links exist.
        if let Some(pos) = line[col..].find(&replacement.old_pattern) {
            let actual_pos = col + pos;
            let end_pos = actual_pos + replacement.old_pattern.len();
            let new_line = format!(
                "{}{}{}",
                &line[..actual_pos],
                replacement.new_pattern,
                &line[end_pos..]
            );
            lines[replacement.line - 1] = new_line;
        } else {
            return Err(MdrefError::Path(format!(
                "Could not find link '{}' in line {} of file {}",
                replacement.old_pattern,
                replacement.line,
                file_path.display()
            )));
        }
    }

    // Reconstruct the content, preserving the original trailing newline if present.
    let mut new_content = lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }
    fs::write(file_path, new_content)?;

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
    use tempfile::TempDir;

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
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("to.md");
        write_file(from.to_str().unwrap(), "");
        write_file(to.to_str().unwrap(), "");

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("sub").join("from.md");
        let to = temp_dir.path().join("to.md");
        write_file(from.to_str().unwrap(), "");
        write_file(to.to_str().unwrap(), "");

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_child_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("sub").join("to.md");
        write_file(from.to_str().unwrap(), "");
        write_file(to.to_str().unwrap(), "");

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("sub/to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_deep_cross_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("a").join("b").join("from.md");
        let to = temp_dir.path().join("x").join("y").join("to.md");
        write_file(from.to_str().unwrap(), "");
        write_file(to.to_str().unwrap(), "");

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../../x/y/to.md"));
    }

    #[test]
    fn test_relative_path_nonexistent_target() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        write_file(from.to_str().unwrap(), "");

        let ghost = temp_dir.path().join("ghost.md");
        let result = relative_path(&from, &ghost);
        assert!(result.is_err());
    }

    // ============= apply_replacements tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_basic() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](old.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](new.md)"));
        assert!(!content.contains("](old.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_preserves_other_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(
            file_path.to_str().unwrap(),
            "# Title\n\nSome text [Link](old.md) more text.\n\nAnother paragraph.",
        );

        let replacements = vec![LinkReplacement {
            line: 3,
            column: 11,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Title"));
        assert!(content.contains("Some text [Link](new.md) more text."));
        assert!(content.contains("Another paragraph."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_line_out_of_range() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "Single line");

        let replacements = vec![LinkReplacement {
            line: 999,
            column: 1,
            old_pattern: "](link.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        let result = apply_replacements(&file_path, &replacements);
        assert!(result.is_err());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_with_subdirectory_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](sub/old.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](sub/old.md)".to_string(),
            new_pattern: "](other/new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](other/new.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_only_replaces_target_link() {
        // Verify that when two identical links exist on the same line,
        // only the one at the specified column is replaced.
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[A](doc.md) and [B](doc.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](doc.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(
            content.contains("[A](new.md)"),
            "Expected [A](new.md) in content: {}",
            content
        );
        assert!(
            content.contains("[B](doc.md)"),
            "Bug: [B](doc.md) was incorrectly modified. Content: {}",
            content
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_multiple_in_same_file() {
        // Verify that multiple replacements in the same file are applied correctly
        // in a single read-write cycle.
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(
            file_path.to_str().unwrap(),
            "[Link1](old.md)\n\n[Link2](old.md)\n\n[Link3](old.md)",
        );

        let replacements = vec![
            LinkReplacement {
                line: 1,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
            LinkReplacement {
                line: 3,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
            LinkReplacement {
                line: 5,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
        ];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.contains("](old.md)"));
        assert_eq!(content.matches("](new.md)").count(), 3);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_preserves_trailing_newline() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](old.md)\n");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](new.md)"));
        assert!(content.ends_with('\n'), "Trailing newline was lost");
    }

    // ============= update via relative_path + apply_replacements tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_same_directory() {
        let temp_dir = TempDir::new().unwrap();
        let ref_file = temp_dir.path().join("ref.md");
        let new_target = temp_dir.path().join("new_target.md");
        write_file(ref_file.to_str().unwrap(), "[Link](old_target.md)");
        write_file(new_target.to_str().unwrap(), "");

        let new_link_path = relative_path(&ref_file, &new_target).unwrap();
        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old_target.md)".to_string(),
            new_pattern: format!("]({})", new_link_path.display()),
        }];

        apply_replacements(&ref_file, &replacements).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("new_target.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_cross_directory() {
        let temp_dir = TempDir::new().unwrap();
        let ref_file = temp_dir.path().join("ref.md");
        let new_target = temp_dir.path().join("sub").join("new_target.md");
        write_file(ref_file.to_str().unwrap(), "[Link](old.md)");
        write_file(new_target.to_str().unwrap(), "");

        let new_link_path = relative_path(&ref_file, &new_target).unwrap();
        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: format!("]({})", new_link_path.display()),
        }];

        apply_replacements(&ref_file, &replacements).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("sub/new_target.md"));
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

    // ============= build_link_replacement with external URL =============

    // ============= build_link_replacement with anchored internal links =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_preserves_anchor() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let other = temp_dir.path().join("other.md");
        let target = temp_dir.path().join("sub").join("target.md");
        write_file(source.to_str().unwrap(), "[Details](other.md#details)");
        write_file(other.to_str().unwrap(), "# Other");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "other.md#details".to_string());

        let result = build_link_replacement(&reference, &source, &target).unwrap();
        assert!(
            result.is_some(),
            "Should produce a replacement for anchored link"
        );

        let replacement = result.unwrap();
        assert!(
            replacement.new_pattern.contains("#details"),
            "Anchor should be preserved in new pattern. Got: {}",
            replacement.new_pattern
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_broken_link() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[Broken](nonexistent.md)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "nonexistent.md".to_string());

        // Should return Ok(None) for broken links, not Err
        let result = build_link_replacement(&reference, &source, &target);
        assert!(
            result.is_ok(),
            "build_link_replacement should not error on broken links: {:?}",
            result.err()
        );
        assert!(
            result.unwrap().is_none(),
            "Broken links should be skipped (return None)"
        );
    }

    // ============= build_link_replacement with pure anchor links =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_pure_anchor_link() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("sub").join("target.md");
        write_file(source.to_str().unwrap(), "[Section](#section)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "#section".to_string());

        // Pure anchor links are internal to the file and should not be rewritten
        let result = build_link_replacement(&reference, &source, &target).unwrap();
        assert!(
            result.is_none(),
            "Pure anchor link (#section) should be skipped, but got: {:?}",
            result.map(|r| r.new_pattern)
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_pure_anchor_with_complex_fragment() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[TOC](#table-of-contents)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "#table-of-contents".to_string());

        let result = build_link_replacement(&reference, &source, &target).unwrap();
        assert!(
            result.is_none(),
            "Pure anchor link (#table-of-contents) should be skipped"
        );
    }

    // ============= build_link_replacement with external URL =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_external_url() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[Google](https://google.com)");
        write_file(target.to_str().unwrap(), "[Google](https://google.com)");

        let reference = Reference::new(target.clone(), 1, 1, "https://google.com".to_string());

        // Should return None — external URL is skipped
        let result = build_link_replacement(&reference, &source, &target).unwrap();
        assert!(result.is_none());

        // Content should remain unchanged
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("https://google.com"));
    }
}
