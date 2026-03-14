use comrak::Arena;
use comrak::nodes::{AstNode, NodeValue};
use comrak::parse_document;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    // If no target specified, accept all links (used for collecting all links)
    let target = match target_canonical {
        Some(t) => t,
        None => return true,
    };

    // Early check: if target is a file, the link's filename must match
    if target.is_file() {
        let link_path = Path::new(link);
        if link_path.file_name() != target.file_name() {
            return false;
        }
    }

    // Resolve and canonicalize the link path
    let canonical_link = match resolve_and_canonicalize_link(file_path, link) {
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

    // Helper: create a temporary directory with test files
    #[allow(clippy::unwrap_used)]
    fn setup_test_dir(name: &str) -> String {
        let dir = format!("test_core_find_{}", name);
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

    // ============= resolve_link tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_relative_existing_file() {
        let dir = setup_test_dir("resolve_rel");
        let base = format!("{}/base.md", dir);
        let target = format!("{}/target.md", dir);
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(Path::new(&base), Path::new("target.md"));
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("target.md"));

        teardown_test_dir(&dir);
    }

    #[test]
    fn test_resolve_link_relative_nonexistent_file() {
        let dir = setup_test_dir("resolve_nonexist");
        let base = format!("{}/base.md", dir);
        write_file(&base, "");

        let result = resolve_link(Path::new(&base), Path::new("ghost.md"));
        assert!(result.is_none());

        teardown_test_dir(&dir);
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
        let dir = setup_test_dir("resolve_nested");
        let base = format!("{}/sub/base.md", dir);
        let target = format!("{}/target.md", dir);
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(Path::new(&base), Path::new("../target.md"));
        assert!(result.is_some());

        teardown_test_dir(&dir);
    }

    // ============= match_link_to_target tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_file_match() {
        let dir = setup_test_dir("match_file");
        let file_path = format!("{}/file.md", dir);
        write_file(&file_path, "");

        let canonical = Path::new(&file_path).canonicalize().unwrap();
        assert!(match_link_to_target(&canonical, &canonical));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_file_mismatch() {
        let dir = setup_test_dir("match_mismatch");
        let file_a = format!("{}/a.md", dir);
        let file_b = format!("{}/b.md", dir);
        write_file(&file_a, "");
        write_file(&file_b, "");

        let canonical_a = Path::new(&file_a).canonicalize().unwrap();
        let canonical_b = Path::new(&file_b).canonicalize().unwrap();
        assert!(!match_link_to_target(&canonical_a, &canonical_b));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_directory() {
        let dir = setup_test_dir("match_dir");
        let sub_file = format!("{}/sub/file.md", dir);
        write_file(&sub_file, "");

        let canonical_file = Path::new(&sub_file).canonicalize().unwrap();
        let canonical_dir = Path::new(&dir).canonicalize().unwrap();
        assert!(match_link_to_target(&canonical_file, &canonical_dir));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_match_link_to_target_outside_directory() {
        let dir = setup_test_dir("match_outside");
        let inside = format!("{}/inside.md", dir);
        write_file(&inside, "");

        let other_dir = setup_test_dir("match_outside_other");
        let outside = format!("{}/outside.md", other_dir);
        write_file(&outside, "");

        let canonical_outside = Path::new(&outside).canonicalize().unwrap();
        let canonical_dir = Path::new(&dir).canonicalize().unwrap();
        assert!(!match_link_to_target(&canonical_outside, &canonical_dir));

        teardown_test_dir(&dir);
        teardown_test_dir(&other_dir);
    }

    // ============= process_link tests =============

    #[test]
    fn test_process_link_no_target_accepts_all() {
        assert!(process_link(Path::new("any.md"), None, "anything"));
        assert!(process_link(
            Path::new("any.md"),
            None,
            "https://example.com"
        ));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_filename_mismatch_early_return() {
        let dir = setup_test_dir("proc_mismatch");
        let target = format!("{}/target.md", dir);
        write_file(&target, "");

        let canonical = Path::new(&target).canonicalize().unwrap();
        // Link has a different filename, should return false early
        assert!(!process_link(
            Path::new(&format!("{}/base.md", dir)),
            Some(&canonical),
            "other.md"
        ));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_matching_link() {
        let dir = setup_test_dir("proc_match");
        let base = format!("{}/base.md", dir);
        let target = format!("{}/target.md", dir);
        write_file(&base, "");
        write_file(&target, "");

        let canonical = Path::new(&target).canonicalize().unwrap();
        assert!(process_link(
            Path::new(&base),
            Some(&canonical),
            "target.md"
        ));

        teardown_test_dir(&dir);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_process_link_unresolvable_link() {
        let dir = setup_test_dir("proc_unresolvable");
        let base = format!("{}/base.md", dir);
        let target = format!("{}/target.md", dir);
        write_file(&base, "");
        write_file(&target, "");

        let canonical = Path::new(&target).canonicalize().unwrap();
        // Link points to a file that doesn't exist
        assert!(!process_link(
            Path::new(&base),
            Some(&canonical),
            "nonexistent.md"
        ));

        teardown_test_dir(&dir);
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
    fn test_process_md_file_external_urls() {
        let content = "[Google](https://google.com)\n[GitHub](https://github.com)";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].link_text, "https://google.com");
        assert_eq!(results[1].link_text, "https://github.com");
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
}
