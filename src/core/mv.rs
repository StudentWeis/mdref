use std::fs;
use std::path::{Path, PathBuf};

use crate::{Reference, find_links};

use super::find::find_references;
use pathdiff::diff_paths;

/// Move references from the old file path to the new file path within Markdown files in the specified root directory.
/// This function finds all references to the old file and updates them to point to the new file.
pub fn mv_file(raw_filepath: &Path, new_filepath: &Path, root: &Path) {
    // Ensure the parent directory of the new file path exists.
    if let Some(parent) = new_filepath.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!("Error creating directories for {}: {}", parent.display(), e);
        return;
    }

    // Copy the original file to the new file path.
    if let Err(e) = fs::copy(raw_filepath, new_filepath) {
        eprintln!(
            "Error copying file from {} to {}: {}",
            raw_filepath.display(),
            new_filepath.display(),
            e
        );
        return;
    }

    let references = find_references(raw_filepath, root);
    match references {
        Ok(refs) => {
            // Update all references to point to the new file path.
            for r in refs {
                update_reference(&r, raw_filepath, new_filepath);
            }
        }
        Err(e) => eprintln!("Error finding references: {}", e),
    }

    let links = find_links(new_filepath);
    match links {
        Ok(links) => {
            for r in links {
                update_link(&r, raw_filepath, new_filepath);
            }
        }
        Err(e) => eprintln!("Error finding links: {}", e),
    }

    // Remove the original file after updating references.
    if let Err(e) = fs::remove_file(raw_filepath) {
        eprintln!(
            "Error removing original file {}: {}",
            raw_filepath.display(),
            e
        );
    }
}

fn update_link(r: &Reference, raw_filepath: &Path, new_filepath: &Path) {
    let current_link_absolute_path = raw_filepath
        .parent()
        .unwrap()
        .join(&r.link_text)
        .canonicalize()
        .unwrap();
    let new_file_absolute_path = new_filepath.canonicalize().unwrap();

    // Skip updating links that point to the file itself.
    if current_link_absolute_path.eq(&raw_filepath.canonicalize().unwrap_or_default()) {
        return;
    }

    let new_link_path = relative_path(&new_file_absolute_path, &current_link_absolute_path);
    let content = match fs::read_to_string(&r.path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file {}: {}", r.path.display(), e);
            return;
        }
    };
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if r.line > lines.len() {
        eprintln!(
            "Line number {} out of range for file {}",
            r.line,
            r.path.display()
        );
        return;
    }
    let line = &lines[r.line - 1];
    let old_pattern = format!("]({})", r.link_text);
    let new_pattern = format!("]({})", new_link_path.display());
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

fn update_reference(r: &Reference, raw_filepath: &Path, new_filepath: &Path) {
    // Skip updating references in the file itself.
    if r.path.canonicalize().unwrap_or_default() == raw_filepath.canonicalize().unwrap_or_default()
    {
        return;
    }

    let current_file_path = &r.path;
    // Compute the relative path from the current file to the new file location.
    let new_link_path = relative_path(current_file_path, new_filepath);
    let content = match fs::read_to_string(&r.path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file {}: {}", r.path.display(), e);
            return;
        }
    };
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if r.line > lines.len() {
        eprintln!(
            "Line number {} out of range for file {}",
            r.line,
            r.path.display()
        );
        return;
    }
    let line = &lines[r.line - 1];
    let old_pattern = format!("]({})", r.link_text);
    let new_pattern = format!("]({})", new_link_path.display());
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

fn relative_path(from: &Path, to: &Path) -> PathBuf {
    diff_paths(
        to.canonicalize().unwrap_or_default(),
        from.parent().unwrap().canonicalize().unwrap_or_default(),
    )
    .unwrap_or_default()
}
