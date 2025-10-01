use std::fs;
use std::path::{Path, PathBuf};

use crate::{Reference, find_links, find_references};
use pathdiff::diff_paths;

/// Move references from the old file path to the new file path within Markdown files in the specified root directory.
/// This function finds all references to the old file and updates them to point to the new file.
/// Moreover, it updates links within the moved file itself to ensure they remain valid.
pub fn mv_file<P, B, D>(raw_file_path: P, new_file_path: B, root_dir: D)
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let raw_file_path = raw_file_path.as_ref();
    let new_file_path = new_file_path.as_ref();
    let root_dir = root_dir.as_ref();

    // Ensure the parent directory of the new file path exists.
    if let Some(parent) = new_file_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        eprintln!("Error creating directories for {}: {}", parent.display(), e);
        return;
    }

    // Find all references to the old file path within the specified root directory.
    let references = find_references(raw_file_path, root_dir);

    // Copy the original file to the new file path.
    // We copy first to avoid data loss in case of errors during reference updates.
    if let Err(e) = fs::copy(raw_file_path, new_file_path) {
        eprintln!(
            "Error copying file from {} to {}: {}",
            raw_file_path.display(),
            new_file_path.display(),
            e
        );
        return;
    }

    match references {
        Ok(refs) => {
            // Update all references to point to the new file path.
            for r in refs {
                update_reference(&r, new_file_path);
            }
        }
        Err(e) => eprintln!("Error finding references: {}", e),
    }

    let links = find_links(new_file_path);
    match links {
        Ok(links) => {
            for r in links {
                update_link(&r, raw_file_path, new_file_path);
            }
        }
        Err(e) => eprintln!("Error finding links: {}", e),
    }

    // Remove the original file after updating references.
    if let Err(e) = fs::remove_file(raw_file_path) {
        eprintln!(
            "Error removing original file {}: {}",
            raw_file_path.display(),
            e
        );
    }
}

/// Update a link within a Markdown file to point to the new file location.
fn update_link(r: &Reference, raw_filepath: &Path, new_filepath: &Path) {
    let current_link_absolute_path = raw_filepath
        .parent()
        .unwrap()
        .join(&r.link_text)
        .canonicalize()
        .unwrap();
    let new_file_absolute_path = new_filepath.canonicalize().unwrap();

    let new_link_path =
        if current_link_absolute_path.eq(&raw_filepath.canonicalize().unwrap_or_default()) {
            PathBuf::from(new_file_absolute_path.file_name().unwrap())
        } else {
            relative_path(&new_file_absolute_path, &current_link_absolute_path)
        };
    replace_link_in_file(r, new_link_path);
}

/// Update a reference within a Markdown file to point to the new file location.
fn update_reference(r: &Reference, new_filepath: &Path) {
    let current_file_path = &r.path;
    let new_link_path = relative_path(current_file_path, new_filepath);
    replace_link_in_file(r, new_link_path);
}

/// Replace the old link in the specified file with the new link path.
fn replace_link_in_file(r: &Reference, new_link_path: PathBuf) {
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

/// Compute the relative path from one file to another.
fn relative_path(from: &Path, to: &Path) -> PathBuf {
    diff_paths(
        to.canonicalize().unwrap_or_default(),
        from.parent().unwrap().canonicalize().unwrap_or_default(),
    )
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_link() {
        todo!()
    }
}
