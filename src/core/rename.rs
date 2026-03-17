use std::path::Path;

use crate::{Result, mv};

/// Rename a file by changing only its filename while keeping it in the same directory.
/// This is a convenience wrapper around `mv` that handles the common case of
/// renaming a file in place.
///
/// # Arguments
///
/// * `source` - Path to the file to rename
/// * `name` - The new filename (not a path, just the filename)
/// * `root` - Root directory to search for references
/// * `dry_run` - If true, only preview changes without making them
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
///
/// # Example
///
/// ```ignore
/// use mdref::rename;
///
/// // Rename "old.md" to "new.md" in the same directory
/// rename("docs/old.md", "new.md", ".", false)?;
/// ```
pub fn rename<P, B, D>(source: P, name: B, root: D, dry_run: bool) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<str>,
    D: AsRef<Path>,
{
    let source = source.as_ref();
    let name = name.as_ref();
    let root = root.as_ref();

    // Compute the new path by replacing only the filename
    let new_path = source.with_file_name(name);

    mv(source, new_path, root, dry_run)
}
