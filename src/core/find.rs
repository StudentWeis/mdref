use rayon::prelude::*;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::{Reference, Result};

/// Regular expression to match Markdown links of the form `[text](link)`.
static LINK_REGEX: &str = r"\[([^\]]*)\]\(([^)]+)\)";

/// Find all references to a given file within Markdown files in the specified root directory.
/// Returns a vector of References containing the referencing file path, line number, column number, and the link text.
pub fn find_references<P, B>(path: P, root_dir: B) -> Result<Vec<Reference>>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
{
    let canonical_path = path.as_ref().canonicalize()?;
    let link_regex = Regex::new(LINK_REGEX).unwrap();
    Ok(WalkDir::new(root_dir)
        .into_iter()
        .par_bridge()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter_map(move |entry| {
            fs::read_to_string(entry.path()).ok().map(|content| {
                process_md_file(&content, entry.path(), &link_regex, Some(&canonical_path))
            })
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
    let link_regex = Regex::new(LINK_REGEX).unwrap();
    let content = fs::read_to_string(filepath)?;
    Ok(process_md_file(&content, filepath, &link_regex, None))
}

/// Process a single Markdown file's content to find links referencing the target file.
fn process_md_file(
    content: &str,
    file_path: &Path,
    link_regex: &Regex,
    target_canonical: Option<&Path>,
) -> Vec<Reference> {
    let mut results = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        for cap in link_regex.captures_iter(line) {
            if process_link(file_path, target_canonical, &cap[2]) {
                let start_byte = cap.get(0).unwrap().start();
                let column = line
                    .char_indices()
                    .position(|(byte_idx, _)| byte_idx >= start_byte)
                    .unwrap_or(line.chars().count())
                    + 1;
                results.push(Reference::new(
                    file_path.to_path_buf(),
                    line_num + 1,
                    column,
                    cap[2].to_string(),
                ));
            }
        }
    }
    results
}

/// Determine whether a markdown link (found in `file_path`) refers to `target_canonical`.
///
/// - If `target_canonical` is `None`, this function returns `true` (used when simply collecting links).
/// - If `Some(target)`, the link is considered a match only when:
///   1. The file name component of the link equals the target's file name, and
///   2. Resolving the link relative to `file_path` and canonicalizing it yields the same absolute path as `target`.
///
/// Returns `true` when both checks succeed; otherwise `false`.
fn process_link(file_path: &Path, target_canonical: Option<&Path>, link: &str) -> bool {
    if let Some(target) = target_canonical {
        let link_path = Path::new(link);
        // Check if filenames match
        if link_path.file_name().unwrap() != target.file_name().unwrap() {
            return false;
        }
        // Check if absolute paths match
        if let Some(resolved_path) = resolve_link(file_path, link_path) {
            matches!(resolved_path.canonicalize(), Ok(canonical) if canonical == *target)
        } else {
            false
        }
    } else {
        true
    }
}

/// Resolve a link relative to the base file path and root directory.
fn resolve_link(base_path: &Path, link_path: &Path) -> Option<PathBuf> {
    if link_path.is_absolute() {
        Some(link_path.to_path_buf())
    } else {
        // Try relative to the file's directory first
        if let Some(parent) = base_path.parent() {
            let resolved = parent.join(link_path);
            if resolved.exists() {
                return Some(resolved);
            }
        }
        None
    }
}
