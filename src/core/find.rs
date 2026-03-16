use comrak::Arena;
use comrak::nodes::{AstNode, NodeValue};
use comrak::parse_document;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::util::{is_external_url, strip_anchor};
use crate::{Reference, Result};

/// Find all references to a given file within Markdown files in the specified root directory.
/// Returns a vector of References containing the referencing file path, line number, column number, and the link text.
pub fn find_references<P, B>(path: P, root_dir: B) -> Result<Vec<Reference>>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let canonical_path = path.as_ref().canonicalize()?;
    Ok(WalkDir::new(root_dir)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter_map(move |entry| {
            fs::read_to_string(entry.path())
                .ok()
                .map(|content| process_md_file(&content, entry.path(), Some(&canonical_path)))
        })
        .flatten()
        .collect())
}

/// Process a single Markdown file to find any file links.
pub fn find_links<P: AsRef<Path>>(filepath: P) -> Result<Vec<Reference>> {
    let filepath = filepath.as_ref();

    // Only markdown files are processed.
    if filepath.extension().and_then(|s| s.to_str()) != Some("md") {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(filepath)?;
    Ok(process_md_file(&content, filepath, None))
}

/// Process a single Markdown file's content to find links referencing the target file.
fn process_md_file(
    content: &str,
    file_path: &Path,
    target_canonical: Option<&Path>,
) -> Vec<Reference> {
    let arena = Arena::new();
    let root = parse_document(&arena, content, &comrak::Options::default());

    let mut results = Vec::new();
    collect_links(root, file_path, target_canonical, &mut results);
    results
}

/// Recursively collect links from the AST.
fn collect_links<'a>(
    node: &'a AstNode<'a>,
    file_path: &Path,
    target_canonical: Option<&Path>,
    results: &mut Vec<Reference>,
) {
    let data = node.data.borrow();

    match &data.value {
        NodeValue::Link(link) => {
            let url = &link.url;
            if process_link(file_path, target_canonical, url) {
                // Get line and column from sourcepos
                let line = data.sourcepos.start.line;
                let column = data.sourcepos.start.column;
                results.push(Reference::new(
                    file_path.to_path_buf(),
                    line,
                    column,
                    url.clone(),
                ));
            }
        }
        NodeValue::Image(image) => {
            let url = &image.url;
            if process_link(file_path, target_canonical, url) {
                // Get line and column from sourcepos
                let line = data.sourcepos.start.line;
                let column = data.sourcepos.start.column;
                results.push(Reference::new(
                    file_path.to_path_buf(),
                    line,
                    column,
                    url.clone(),
                ));
            }
        }
        _ => {}
    }

    for child in node.children() {
        collect_links(child, file_path, target_canonical, results);
    }
}

/// Determine whether a markdown link (found in `file_path`) refers to `target_canonical`.
///
/// - If `target_canonical` is `None`, this function returns `true` (used when simply collecting links).
/// - If `Some(target)`, the link is considered a match if:
///   - For files: the file name matches and the resolved path equals the target.
///   - For directories: the resolved path is inside the target directory.
///
/// Returns `true` when the checks succeed; otherwise `false`.
fn process_link(file_path: &Path, target_canonical: Option<&Path>, link: &str) -> bool {
    // External URLs (https://, http://, ftp://, etc.) are not local file paths
    // and should not be matched as file references.
    if is_external_url(link) {
        return false;
    }

    // Strip anchor from link for file path resolution
    let link_without_anchor = match strip_anchor(link) {
        Some(l) => l,
        None => return false, // Pure anchor links don't reference external files
    };

    // If no target specified, accept all links (used for collecting all links)
    let target = match target_canonical {
        Some(t) => t,
        None => return true,
    };

    // Early check: if target is a file, the link's filename must match
    if target.is_file() {
        let link_path = Path::new(link_without_anchor);
        if link_path.file_name() != target.file_name() {
            return false;
        }
    }

    // Resolve and canonicalize the link path
    let canonical_link = match resolve_and_canonicalize_link(file_path, link_without_anchor) {
        Some(path) => path,
        None => return false,
    };

    // Match the link against the target
    match_link_to_target(&canonical_link, target)
}

/// Resolve a link path and canonicalize it.
///
/// Returns `None` if the link cannot be resolved or canonicalized.
fn resolve_and_canonicalize_link(base_file: &Path, link: &str) -> Option<PathBuf> {
    let link_path = Path::new(link);
    let resolved = resolve_link(base_file, link_path)?;
    resolved.canonicalize().ok()
}

/// Check if a canonicalized link matches the target path.
///
/// For files: the canonical path must match (filename check already done earlier).
/// For directories: the link must resolve to a path inside the target directory.
fn match_link_to_target(canonical_link: &Path, target: &Path) -> bool {
    if target.is_file() {
        // Filename already checked, just compare canonical paths
        canonical_link == target
    } else if target.is_dir() {
        canonical_link.starts_with(target)
    } else {
        false
    }
}

/// Resolve a link relative to the base file path.
///
/// Handles both absolute and relative links.
/// For relative links, resolves them relative to the base file's parent directory.
fn resolve_link(base_path: &Path, link_path: &Path) -> Option<PathBuf> {
    if link_path.is_absolute() {
        return Some(link_path.to_path_buf());
    }

    // Resolve relative to the base file's directory
    let parent = base_path.parent()?;
    let resolved = parent.join(link_path);

    if resolved.exists() {
        Some(resolved)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    // ============= resolve_link tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_relative_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("target.md");
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(&base, Path::new("target.md"));
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("target.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_relative_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        write_file(&base, "");

        let result = resolve_link(&base, Path::new("ghost.md"));
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_link_absolute_path() {
        let absolute = PathBuf::from("/tmp/some_absolute_path.md");
        let result = resolve_link(Path::new("any/base.md"), &absolute);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), absolute);
    }

    #[test]
    fn test_resolve_link_no_parent() {
        // A bare filename with no parent directory
        let result = resolve_link(Path::new(""), Path::new("target.md"));
        assert!(result.is_none());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_nested_relative() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("sub").join("base.md");
        let target = temp_dir.path().join("target.md");
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(&base, Path::new("../target.md"));
        assert!(result.is_some());
    }

    // ============= match_link_to_target tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_file_match() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.md");
        write_file(&file_path, "");

        let canonical = file_path.canonicalize().unwrap();
        assert!(match_link_to_target(&canonical, &canonical));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_file_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let file_a = temp_dir.path().join("a.md");
        let file_b = temp_dir.path().join("b.md");
        write_file(&file_a, "");
        write_file(&file_b, "");

        let canonical_a = file_a.canonicalize().unwrap();
        let canonical_b = file_b.canonicalize().unwrap();
        assert!(!match_link_to_target(&canonical_a, &canonical_b));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sub_file = temp_dir.path().join("sub").join("file.md");
        write_file(&sub_file, "");

        let canonical_file = sub_file.canonicalize().unwrap();
        let canonical_dir = temp_dir.path().canonicalize().unwrap();
        assert!(match_link_to_target(&canonical_file, &canonical_dir));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_outside_directory() {
        let temp_dir = TempDir::new().unwrap();
        let other_temp_dir = TempDir::new().unwrap();

        let inside = temp_dir.path().join("inside.md");
        let outside = other_temp_dir.path().join("outside.md");
        write_file(&inside, "");
        write_file(&outside, "");

        let canonical_outside = outside.canonicalize().unwrap();
        let canonical_dir = temp_dir.path().canonicalize().unwrap();
        assert!(!match_link_to_target(&canonical_outside, &canonical_dir));
    }

    // ============= process_link tests =============

    #[test]
    fn test_process_link_no_target_accepts_all() {
        assert!(process_link(Path::new("any.md"), None, "anything"));
        // Note: External URLs are filtered out even when no target specified
        assert!(!process_link(
            Path::new("any.md"),
            None,
            "https://example.com"
        ));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_filename_mismatch_early_return() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("target.md");
        write_file(&target, "");

        let canonical = target.canonicalize().unwrap();
        // Link has a different filename, should return false early
        assert!(!process_link(
            &temp_dir.path().join("base.md"),
            Some(&canonical),
            "other.md"
        ));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_matching_link() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("target.md");
        write_file(&base, "");
        write_file(&target, "");

        let canonical = target.canonicalize().unwrap();
        assert!(process_link(&base, Some(&canonical), "target.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_unresolvable_link() {
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("target.md");
        write_file(&base, "");
        write_file(&target, "");

        let canonical = target.canonicalize().unwrap();
        // Link points to a file that doesn't exist
        assert!(!process_link(&base, Some(&canonical), "nonexistent.md"));
    }

    #[test]
    fn test_process_link_filters_external_url() {
        // External URLs should be filtered out regardless of target
        assert!(!process_link(
            Path::new("any.md"),
            None,
            "https://google.com"
        ));
        assert!(!process_link(
            Path::new("any.md"),
            None,
            "http://example.com"
        ));
        assert!(!process_link(
            Path::new("any.md"),
            None,
            "ftp://files.example.com/doc.md"
        ));
    }

    // ============= process_md_file tests =============

    #[test]
    fn test_process_md_file_no_links() {
        let content = "# Title\n\nJust plain text, no links here.";
        let results = process_md_file(content, Path::new("test.md"), None);
        assert!(results.is_empty());
    }

    #[test]
    fn test_process_md_file_collects_all_links() {
        let content = "[Link1](a.md)\n\n[Link2](b.md)\n\n![Image](c.png)";
        let results = process_md_file(content, Path::new("test.md"), None);
        assert_eq!(results.len(), 3);

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(link_texts.contains(&"a.md"));
        assert!(link_texts.contains(&"b.md"));
        assert!(link_texts.contains(&"c.png"));
    }

    #[test]
    fn test_process_md_file_line_numbers() {
        let content = "[First](a.md)\n\n[Second](b.md)";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line, 1);
        assert_eq!(results[1].line, 3);
    }

    #[test]
    fn test_process_md_file_multiple_links_same_line() {
        let content = "[A](a.md) and [B](b.md) and [C](c.md)";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 3);
        // All on line 1
        assert!(results.iter().all(|r| r.line == 1));
        // Columns should be distinct
        let columns: Vec<usize> = results.iter().map(|r| r.column).collect();
        assert!(columns[0] < columns[1] && columns[1] < columns[2]);
    }

    #[test]
    fn test_process_md_file_image_links() {
        let content = "![Alt text](image.png)\n\n![Another](photo.jpg)";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].link_text, "image.png");
        assert_eq!(results[1].link_text, "photo.jpg");
    }

    #[test]
    fn test_process_md_file_external_urls_filtered() {
        // External URLs should be filtered out and not included in results
        let content = "[Google](https://google.com)\n[GitHub](https://github.com)";
        let results = process_md_file(content, Path::new("test.md"), None);

        // External URLs are now filtered out
        assert!(results.is_empty(), "External URLs should be filtered out");
    }

    #[test]
    fn test_process_md_file_mixed_content() {
        let content = "# Title\n\nSome text [link](file.md) more text.\n\n> Quote with ![img](pic.png)\n\n- List item [ref](other.md)";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 3);
        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(link_texts.contains(&"file.md"));
        assert!(link_texts.contains(&"pic.png"));
        assert!(link_texts.contains(&"other.md"));
    }

    #[test]
    fn test_process_md_file_pure_anchor_filtered() {
        // Pure anchor links should be filtered out
        let content = "[Section](#section)\n[TOC](#table-of-contents)";
        let results = process_md_file(content, Path::new("test.md"), None);

        // Pure anchor links are filtered out
        assert!(
            results.is_empty(),
            "Pure anchor links should be filtered out"
        );
    }
}
