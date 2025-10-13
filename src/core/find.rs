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
