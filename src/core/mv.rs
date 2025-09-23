use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::find::find_references;
use pathdiff::diff_paths;

/// Move references from the old file path to the new file path within Markdown files in the specified root directory.
/// This function finds all references to the old file and updates them to point to the new file.
pub fn mv_references(raw_filepath: &Path, new_filepath: &Path, root: &Path) {
    let references = find_references(raw_filepath, root);
    match references {
        Ok(refs) => {
            for r in refs {
                let ref_dir = &r.path;
                let new_abs = if new_filepath.is_absolute() {
                    new_filepath.to_path_buf()
                } else {
                    env::current_dir().unwrap().join(new_filepath)
                };
                let new_link = relative_path(ref_dir.parent().unwrap(), &new_abs);
                let content = match fs::read_to_string(&r.path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading file {}: {}", r.path.display(), e);
                        continue;
                    }
                };
                let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
                if r.line > lines.len() {
                    eprintln!(
                        "Line number {} out of range for file {}",
                        r.line,
                        r.path.display()
                    );
                    continue;
                }
                let line = &lines[r.line - 1];
                let old_pattern = format!("]({})", r.link_text);
                let new_pattern = format!("]({})", new_link.display());
                if line.contains(&old_pattern) {
                    let new_line = line.replace(&old_pattern, &new_pattern);
                    lines[r.line - 1] = new_line;
                    let new_content = lines.join("\n");
                    if let Err(e) = fs::write(&r.path, new_content) {
                        eprintln!("Error writing file {}: {}", r.path.display(), e);
                    }
                } else {
                    eprintln!(
                        "Could not find link in line {} of file {}",
                        r.line,
                        r.path.display()
                    );
                }
            }
        }
        Err(e) => eprintln!("Error finding references: {}", e),
    }
    // Move the original file to the new file path.
    if let Err(e) = fs::rename(raw_filepath, new_filepath) {
        eprintln!(
            "Error moving file from {} to {}: {}",
            raw_filepath.display(),
            new_filepath.display(),
            e
        );
    }
}

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let diff = diff_paths(
        to.canonicalize().unwrap_or_default(),
        from.canonicalize().unwrap_or_default(),
    )
    .unwrap_or_default();
    println!(
        "Relative path from {} to {}: {}",
        from.display(),
        to.display(),
        diff.display()
    );
    diff
}

#[test]
fn test_mv_references() {
    let old_filepath = Path::new("examples/main.md");
    let new_filepath = Path::new("examples/renamed_main.md");
    let root = Path::new("examples");
    mv_references(old_filepath, new_filepath, root);
}
