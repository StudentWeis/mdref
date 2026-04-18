use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use comrak::{
    Arena,
    nodes::{AstNode, NodeValue},
    parse_document,
};
use indicatif::ProgressBar;
use rayon::prelude::*;

use super::util::{
    collect_markdown_files, is_external_url, strip_anchor, strip_utf8_bom_prefix, url_decode_link,
};
use crate::{Reference, Result};

/// Find all references to a given file within Markdown files in the specified root directory.
/// Returns a vector of References containing the referencing file path, line number, column number, and the link text.
pub fn find_references<P, B>(path: P, root_dir: B) -> Result<Vec<Reference>>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    find_references_with_progress(path, root_dir, None)
}

/// Find all references with an optional progress bar.
///
/// When a `ProgressBar` is provided, it is incremented once for each Markdown file scanned.
/// The caller is responsible for creating and finishing the progress bar.
pub fn find_references_with_progress<P, B>(
    path: P,
    root_dir: B,
    progress: Option<&ProgressBar>,
) -> Result<Vec<Reference>>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let canonical_path = path.as_ref().canonicalize()?;
    let markdown_files = collect_markdown_files(root_dir.as_ref());

    if let Some(progress_bar) = progress {
        progress_bar.set_length(markdown_files.len() as u64);
    }

    let results: Vec<Result<Vec<Reference>>> = markdown_files
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)?;
            let refs = process_md_file(&content, path, Some(&canonical_path));
            if let Some(progress_bar) = progress {
                progress_bar.inc(1);
            }
            Ok(refs)
        })
        .collect();

    let mut references = Vec::new();
    for result in results {
        references.extend(result?);
    }

    Ok(references)
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
    let ignored_lines = collect_ignored_reference_definition_lines(root);

    // Step 1: Collect link reference definitions from raw text.
    // These are not represented as AST nodes by comrak, so we scan raw text
    // but skip source ranges that comrak identified as code blocks.
    let ref_defs = parse_link_reference_definitions(content, &ignored_lines);

    // Step 2: Collect inline links from the AST, skipping those that are
    // reference-style links (their URL comes from a definition line).
    // We detect reference-style links by checking if the source text at the
    // node's position contains the `](` pattern (inline) vs `][` pattern (reference).
    let mut results = Vec::new();
    collect_links(root, file_path, target_canonical, content, &mut results);

    // Step 3: Add reference definitions as References.
    for (line_number, url, column) in &ref_defs {
        if process_link(file_path, target_canonical, url) {
            results.push(Reference::with_link_type(
                file_path.to_path_buf(),
                *line_number,
                *column,
                url.clone(),
                crate::LinkType::ReferenceDefinition,
            ));
        }
    }

    results
}

/// Parse link reference definitions from raw Markdown text.
///
/// A link reference definition has the form:
///   `[label]: URL` or `[label]: <URL>` with optional title.
///
/// Returns a vector of `(line_number, url, column)` tuples.
/// `line_number` is 1-based. `column` is the 1-based column of the `[` character.
fn parse_link_reference_definitions(
    content: &str,
    ignored_lines: &HashSet<usize>,
) -> Vec<(usize, String, usize)> {
    let mut definitions = Vec::new();

    for (line_index, line) in content.lines().enumerate() {
        let line_number = line_index + 1;
        if ignored_lines.contains(&line_number) {
            continue;
        }

        let (line, bom_offset) = strip_utf8_bom_prefix(line);

        // Link reference definitions may have up to 3 leading spaces.
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();
        if leading_spaces > 3 {
            continue;
        }

        // Must start with `[label]:` pattern
        if !trimmed.starts_with('[') {
            continue;
        }

        // Find the closing `]` of the label
        let label_end = match trimmed.find("]:") {
            Some(pos) => pos,
            None => continue,
        };

        // Ensure the label is not empty
        let label = &trimmed[1..label_end];
        if label.is_empty() {
            continue;
        }

        // Extract the URL part after `]: `
        let after_colon = &trimmed[label_end + 2..];
        let after_colon = after_colon.trim_start();
        if after_colon.is_empty() {
            continue;
        }

        // Handle angle-bracket URLs: `<URL>`
        let url = if after_colon.starts_with('<') {
            match after_colon.find('>') {
                Some(end) => after_colon[1..end].to_string(),
                None => continue,
            }
        } else {
            // URL is the first non-whitespace token
            after_colon
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        };

        if url.is_empty() {
            continue;
        }

        // Column is 1-based, pointing to the `[` character
        let column = bom_offset + leading_spaces + 1;
        definitions.push((line_number, url, column));
    }

    definitions
}

fn collect_ignored_reference_definition_lines<'a>(root: &'a AstNode<'a>) -> HashSet<usize> {
    let mut ignored_lines = HashSet::new();
    collect_code_block_lines(root, &mut ignored_lines);
    ignored_lines
}

fn collect_code_block_lines<'a>(node: &'a AstNode<'a>, ignored_lines: &mut HashSet<usize>) {
    let data = node.data.borrow();
    let sourcepos = data.sourcepos;
    let is_code_block = matches!(data.value, NodeValue::CodeBlock(_));
    drop(data);

    if is_code_block && sourcepos.start.line > 0 && sourcepos.end.line >= sourcepos.start.line {
        for line in sourcepos.start.line..=sourcepos.end.line {
            ignored_lines.insert(line);
        }
    }

    for child in node.children() {
        collect_code_block_lines(child, ignored_lines);
    }
}

/// Recursively collect links from the AST.
///
/// For each `NodeValue::Link`, we check the original source text at the node's
/// position to determine whether it is an inline link (`[text](url)`) or a
/// reference-style link (`[text][ref]`).  Reference-style links are skipped
/// here because they will be reported from the definition line instead.
fn collect_links<'a>(
    node: &'a AstNode<'a>,
    file_path: &Path,
    target_canonical: Option<&Path>,
    source_content: &str,
    results: &mut Vec<Reference>,
) {
    let data = node.data.borrow();

    match &data.value {
        NodeValue::Link(link) => {
            let url = &link.url;
            // Determine if this AST link is an inline link or a reference-style link
            // by inspecting the source text at the node's position.
            // Reference-style links are handled via definition-line scanning.
            if !is_reference_style_link(source_content, &data.sourcepos)
                && process_link(file_path, target_canonical, url)
            {
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
        collect_links(child, file_path, target_canonical, source_content, results);
    }
}

/// Check whether an AST link node corresponds to a reference-style link
/// (e.g. `[text][ref]`) rather than an inline link (e.g. `[text](url)`).
///
/// We inspect the source text spanning the node's position. An inline link
/// always contains `](` between the text and URL, while a reference-style
/// link contains `][` instead.
fn is_reference_style_link(source_content: &str, sourcepos: &comrak::nodes::Sourcepos) -> bool {
    let lines: Vec<&str> = source_content.lines().collect();
    let start_line = sourcepos.start.line;
    let end_line = sourcepos.end.line;

    if start_line == 0 || start_line > lines.len() {
        return false;
    }

    // Extract the source text covered by this node's span
    let node_text = if start_line == end_line {
        let line = lines[start_line - 1];
        let start_col = sourcepos.start.column.saturating_sub(1);
        let end_col = sourcepos.end.column.min(line.len());
        &line[start_col..end_col]
    } else {
        // Multi-line node: just check the first line from the start column
        let line = lines[start_line - 1];
        let start_col = sourcepos.start.column.saturating_sub(1);
        &line[start_col..]
    };

    // An inline link has `](` in the node text; a reference-style link does not.
    // Instead, reference-style links have `][` (e.g. `[text][ref]`).
    !node_text.contains("](")
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
/// Also handles URL-encoded characters in the link path (e.g., `%20` for space).
fn resolve_link(base_path: &Path, link_path: &Path) -> Option<PathBuf> {
    if link_path.is_absolute() {
        return Some(link_path.to_path_buf());
    }

    // Resolve relative to the base file's directory
    let parent = base_path.parent()?;

    // Convert link_path to string and decode URL encoding
    let link_str = link_path.to_str()?;
    let decoded_link = url_decode_link(link_str);
    let resolved = parent.join(decoded_link);

    if resolved.exists() {
        Some(resolved)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::test_utils::write_file;

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

    // ============= resolve_link with URL-encoded paths =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_url_encoded_space() {
        // Test that %20 encoded spaces are decoded correctly
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("my file.md");
        write_file(&base, "");
        write_file(&target, "");

        // The link "my%20file.md" should resolve to "my file.md"
        let result = resolve_link(&base, Path::new("my%20file.md"));
        assert!(result.is_some(), "URL-encoded space should be decoded");
        assert!(result.unwrap().ends_with("my file.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_url_encoded_multiple_spaces() {
        // Test multiple encoded spaces in path
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("my document with spaces.md");
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(&base, Path::new("my%20document%20with%20spaces.md"));
        assert!(
            result.is_some(),
            "Multiple URL-encoded spaces should be decoded"
        );
        assert!(result.unwrap().ends_with("my document with spaces.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_url_encoded_in_subdirectory() {
        // Test encoded spaces in subdirectory path
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let subdir = temp_dir.path().join("my docs");
        let target = subdir.join("read me.md");
        write_file(&base, "");
        write_file(&target, "");

        let result = resolve_link(&base, Path::new("my%20docs/read%20me.md"));
        assert!(
            result.is_some(),
            "URL-encoded path with subdirectory should work"
        );
        assert!(result.unwrap().ends_with("read me.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_mixed_encoded_and_plain() {
        // Test path with both encoded and actual spaces
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("my file name.md");
        write_file(&base, "");
        write_file(&target, "");

        // Even if the link has a mix of %20 and actual spaces, it should resolve
        let result = resolve_link(&base, Path::new("my%20file name.md"));
        assert!(result.is_some(), "Mixed encoding should still work");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_plain_space_in_link() {
        // Test link that already contains actual spaces (not encoded)
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let target = temp_dir.path().join("my file.md");
        write_file(&base, "");
        write_file(&target, "");

        // Link with actual space should work directly
        let result = resolve_link(&base, Path::new("my file.md"));
        assert!(result.is_some(), "Plain space in link should work");
        assert!(result.unwrap().ends_with("my file.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_link_plain_space_in_subdirectory() {
        // Test link with spaces in both directory and filename
        let temp_dir = TempDir::new().unwrap();
        let base = temp_dir.path().join("base.md");
        let subdir = temp_dir.path().join("my docs");
        let target = subdir.join("read me.md");
        write_file(&base, "");
        write_file(&target, "");

        // Link with spaces in path should work directly
        let result = resolve_link(&base, Path::new("my docs/read me.md"));
        assert!(result.is_some(), "Plain spaces in path should work");
        assert!(result.unwrap().ends_with("read me.md"));
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

    // ============= link reference definition tests =============

    #[test]
    fn test_process_md_file_link_reference_definition() {
        // Link reference definitions should be collected
        let content = "[text][ref]\n\n[ref]: ./file.md";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert!(
            !results.is_empty(),
            "Link reference definitions should produce results"
        );

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(
            link_texts.contains(&"./file.md"),
            "Should contain the URL from the link reference definition. Got: {:?}",
            link_texts
        );
    }

    #[test]
    fn test_process_md_file_link_reference_definition_line_number() {
        // The reference should point to the definition line, not the usage line
        let content = "[text][ref]\n\n[ref]: ./file.md";
        let results = process_md_file(content, Path::new("test.md"), None);

        // We expect a reference pointing to the definition line (line 3)
        let def_refs: Vec<&Reference> = results
            .iter()
            .filter(|r| r.link_text == "./file.md")
            .collect();
        assert!(
            !def_refs.is_empty(),
            "Should find the link reference definition"
        );

        // The definition is on line 3
        let has_def_line = def_refs.iter().any(|r| r.line == 3);
        assert!(
            has_def_line,
            "Link reference definition should report the definition line (3). Got lines: {:?}",
            def_refs.iter().map(|r| r.line).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_process_md_file_multiple_link_reference_definitions() {
        let content = "[a][ref1]\n[b][ref2]\n\n[ref1]: ./first.md\n[ref2]: ./second.md";
        let results = process_md_file(content, Path::new("test.md"), None);

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(
            link_texts.contains(&"./first.md"),
            "Should contain first ref definition. Got: {:?}",
            link_texts
        );
        assert!(
            link_texts.contains(&"./second.md"),
            "Should contain second ref definition. Got: {:?}",
            link_texts
        );
    }

    #[test]
    fn test_process_md_file_link_reference_definition_with_title() {
        // Link reference definitions can have optional titles
        let content = "[text][ref]\n\n[ref]: ./file.md \"Title\"";
        let results = process_md_file(content, Path::new("test.md"), None);

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(
            link_texts.contains(&"./file.md"),
            "Should contain URL from definition with title. Got: {:?}",
            link_texts
        );
    }

    #[test]
    fn test_process_md_file_link_reference_definition_with_angle_brackets() {
        // Link reference definitions can use angle brackets around URL
        let content = "[text][ref]\n\n[ref]: <./file.md>";
        let results = process_md_file(content, Path::new("test.md"), None);

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(
            link_texts.contains(&"./file.md"),
            "Should contain URL from angle-bracket definition. Got: {:?}",
            link_texts
        );
    }

    #[test]
    fn test_process_md_file_link_reference_definition_external_url_filtered() {
        // External URLs in link reference definitions should be filtered out
        let content = "[text][ref]\n\n[ref]: https://example.com";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert!(
            results.is_empty(),
            "External URLs in link reference definitions should be filtered out"
        );
    }

    #[test]
    fn test_process_md_file_link_reference_definition_no_duplicate_with_inline() {
        // When a link reference definition is used, we should get exactly one reference
        // pointing to the definition line, not a duplicate from the usage site
        let content = "[text][ref]\n\n[ref]: ./file.md";
        let results = process_md_file(content, Path::new("test.md"), None);

        let file_refs: Vec<&Reference> = results
            .iter()
            .filter(|r| r.link_text == "./file.md")
            .collect();
        assert_eq!(
            file_refs.len(),
            1,
            "Should have exactly one reference for the definition, not duplicates. Got: {:?}",
            file_refs
                .iter()
                .map(|r| (r.line, r.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_process_md_file_ignores_link_reference_definition_in_fenced_code_block() {
        let content = "```md\n[text][ref]\n\n[ref]: ./file.md\n```\n";
        let results = process_md_file(content, Path::new("test.md"), None);

        assert!(
            results.is_empty(),
            "Link reference definitions inside fenced code blocks should be ignored. Got: {:?}",
            results
                .iter()
                .map(|r| (&r.link_text, r.line, r.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_process_md_file_keeps_link_reference_definition_outside_fenced_code_block() {
        let content = "```md\n[text][ref]\n\n[ref]: ./ignored.md\n```\n\n[real]: ./file.md\n";
        let results = process_md_file(content, Path::new("test.md"), None);

        let link_texts: Vec<&str> = results.iter().map(|r| r.link_text.as_str()).collect();
        assert!(
            link_texts.contains(&"./file.md"),
            "Should still collect reference definitions outside fenced code blocks. Got: {:?}",
            link_texts
        );
        assert!(
            !link_texts.contains(&"./ignored.md"),
            "Definitions inside fenced code blocks should still be ignored. Got: {:?}",
            link_texts
        );
    }
}
