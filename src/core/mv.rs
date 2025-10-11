use std::fs;
use std::path::{Path, PathBuf};

use crate::{MdrefError, Reference, Result, find_links, find_references};
use pathdiff::diff_paths;

/// Move references from the old file path to the new file path within Markdown files in the specified root directory.
/// This function finds all references to the old file and updates them to point to the new file.
/// Moreover, it updates links within the moved file itself to ensure they remain valid.
pub fn mv_file<P, B, D>(raw_file_path: P, new_file_path: B, root_dir: D) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let raw_file_path = raw_file_path.as_ref();
    let new_file_path = new_file_path.as_ref();
    let root_dir = root_dir.as_ref();

    // Ensure the parent directory of the new file path exists.
    if let Some(parent) = new_file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Find all references to the old file path within the specified root directory.
    let references = find_references(raw_file_path, root_dir)?;

    // Copy the original file to the new file path.
    // We copy first to avoid data loss in case of errors during reference updates.
    fs::copy(raw_file_path, new_file_path)?;

    // Update all references to point to the new file path.
    for r in references {
        update_reference(&r, new_file_path)?;
    }

    let links = find_links(new_file_path)?;
    for r in links {
        update_link(&r, raw_file_path, new_file_path)?;
    }

    // Remove the original file after updating references.
    fs::remove_file(raw_file_path)?;

    Ok(())
}

/// Update a link within a Markdown file to point to the new file location.
fn update_link(r: &Reference, raw_filepath: &Path, new_filepath: &Path) -> Result<()> {
    let current_link_absolute_path = raw_filepath
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?
        .join(&r.link_text)
        .canonicalize()?;
    let new_file_absolute_path = new_filepath.canonicalize()?;

    let new_link_path =
        if current_link_absolute_path.eq(&raw_filepath.canonicalize().unwrap_or_default()) {
            PathBuf::from(
                new_file_absolute_path
                    .file_name()
                    .ok_or_else(|| MdrefError::Path("No file name".to_string()))?,
            )
        } else {
            relative_path(&new_file_absolute_path, &current_link_absolute_path)?
        };
    replace_link_in_file(r, new_link_path)?;
    Ok(())
}

/// Update a reference within a Markdown file to point to the new file location.
fn update_reference(r: &Reference, new_filepath: &Path) -> Result<()> {
    let current_file_path = &r.path;
    let new_link_path = relative_path(current_file_path, new_filepath)?;
    replace_link_in_file(r, new_link_path)?;
    Ok(())
}

/// Replace the old link in the specified file with the new link path.
fn replace_link_in_file(r: &Reference, new_link_path: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&r.path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if r.line > lines.len() {
        return Err(MdrefError::InvalidLine(format!(
            "Line number {} out of range for file {}",
            r.line,
            r.path.display()
        )));
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
            return Err(e.into());
        }
    } else {
        eprintln!(
            "Could not find link in line {} of file {}",
            r.line,
            r.path.display()
        );
    }
    Ok(())
}

/// Compute the relative path from one file to another.
fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let to_canonical = to.canonicalize()?;
    let from_parent = from
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let from_canonical: PathBuf = from_parent.canonicalize()?;
    Ok(diff_paths(to_canonical, from_canonical).unwrap_or_default())
}
