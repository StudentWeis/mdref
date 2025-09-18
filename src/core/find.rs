use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Find all references to a given file within Markdown files in the specified root directory.
/// Returns a vector of tuples containing the referencing file path, line number, and the link text.
pub fn find_references(
    filepath: &Path,
    root: &Path,
) -> Result<Vec<(PathBuf, usize, String)>, std::io::Error> {
    let target_canonical = filepath.canonicalize()?;

    let mut references = Vec::new();
    let link_regex = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();

    // Find all Markdown files and check links
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .for_each(|entry| {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                process_md_file(
                    &content,
                    entry.path(),
                    &link_regex,
                    &target_canonical,
                    &mut references,
                );
            }
        });
    Ok(references)
}

/// Process a single Markdown file's content to find links referencing the target file.
fn process_md_file(
    content: &str,
    file_path: &Path,
    link_regex: &Regex,
    target_canonical: &Path,
    references: &mut Vec<(PathBuf, usize, String)>,
) {
    content.lines().enumerate().for_each(|(line_num, line)| {
        for cap in link_regex.captures_iter(line) {
            process_link(file_path, target_canonical, references, line_num, cap);
        }
    });
}

/// Process a single link match to see if it references the target file.
/// Need to confirm two things:
/// 1. The filenames of both must be identical.
/// 2. The absolute paths of both must be identical.
fn process_link(
    file_path: &Path,
    target_canonical: &Path,
    references: &mut Vec<(PathBuf, usize, String)>,
    line_num: usize,
    cap: regex::Captures<'_>,
) {
    let link = &cap[2];
    let link_path = Path::new(link);
    // Quick check: if the file names don't match, skip
    if link_path.file_name().unwrap() != target_canonical.file_name().unwrap() {
        return;
    }
    // Resolve the link to an absolute path
    if let Some(resolved_path) = resolve_link(file_path, link_path) {
        match resolved_path.canonicalize() {
            Ok(canonical) if canonical == *target_canonical => {
                references.push((file_path.to_path_buf(), line_num + 1, link.to_string()));
            }
            _ => {}
        }
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
