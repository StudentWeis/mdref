use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_references(
    filepath: &Path,
    root: &Path,
) -> Result<Vec<(PathBuf, usize, String)>, std::io::Error> {
    let target_canonical = filepath.canonicalize()?;

    let mut references = Vec::new();
    let link_regex = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            process_md_file(
                &content,
                entry.path(),
                &link_regex,
                root,
                &target_canonical,
                &mut references,
            );
        }
    }

    Ok(references)
}

fn process_md_file(
    content: &str,
    file_path: &Path,
    link_regex: &Regex,
    root: &Path,
    target_canonical: &Path,
    references: &mut Vec<(PathBuf, usize, String)>,
) {
    for (line_num, line) in content.lines().enumerate() {
        for cap in link_regex.captures_iter(line) {
            let link = &cap[2];
            if let Some(resolved_path) = resolve_link(file_path, link, root) {
                match resolved_path.canonicalize() {
                    Ok(canonical) if canonical == *target_canonical => {
                        references.push((file_path.to_path_buf(), line_num + 1, link.to_string()));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn resolve_link(base_path: &Path, link: &str, root: &Path) -> Option<PathBuf> {
    let link_path = Path::new(link);
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
        // If not found, try relative to the root directory
        Some(root.join(link_path))
    }
}
