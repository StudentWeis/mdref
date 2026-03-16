use std::path::Path;

use crate::{Result, mv_file};

/// Rename a file by changing only its filename while keeping it in the same directory.
/// This is a convenience wrapper around `mv_file` that handles the common case of
/// renaming a file in place.
///
/// # Arguments
///
/// * `file_path` - Path to the file to rename
/// * `new_name` - The new filename (not a path, just the filename)
/// * `root_dir` - Root directory to search for references
/// * `dry_run` - If true, only preview changes without making them
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
///
/// # Example
///
/// ```ignore
/// use mdref::rename_file;
///
/// // Rename "old.md" to "new.md" in the same directory
/// rename_file("docs/old.md", "new.md", ".", false)?;
/// ```
pub fn rename_file<P, B, D>(file_path: P, new_name: B, root_dir: D, dry_run: bool) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<str>,
    D: AsRef<Path>,
{
    let file_path = file_path.as_ref();
    let new_name = new_name.as_ref();
    let root_dir = root_dir.as_ref();

    // Compute the new path by replacing only the filename
    let new_path = file_path.with_file_name(new_name);

    mv_file(file_path, new_path, root_dir, dry_run)
}
