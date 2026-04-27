//! Path resolution and validation for move operations.
//!
//! These helpers normalize destination paths (including the "move into existing
//! directory" case) and reject invalid moves early, before any planning or
//! filesystem mutation happens.

use std::path::{Path, PathBuf};

use crate::{MdrefError, Result};

/// Resolve the destination path, handling the case where the destination is an existing directory.
pub(super) fn resolve_destination(source: &Path, destination: &Path) -> Result<PathBuf> {
    if destination.is_dir() {
        let filename = source
            .file_name()
            .ok_or_else(|| MdrefError::PathValidation {
                path: source.to_path_buf(),
                details: "source path has no filename".to_string(),
            })?;
        Ok(destination.join(filename))
    } else {
        Ok(destination.to_path_buf())
    }
}

/// Canonicalize a destination path, handling the case where it doesn't exist yet.
pub(super) fn canonicalize_destination(destination: &Path) -> Result<PathBuf> {
    if destination.exists() {
        return destination
            .canonicalize()
            .map_err(|e| MdrefError::PathValidation {
                path: destination.to_path_buf(),
                details: format!("cannot canonicalize destination path: {e}"),
            });
    }

    let parent = destination
        .parent()
        .ok_or_else(|| MdrefError::PathValidation {
            path: destination.to_path_buf(),
            details: "destination path has no parent directory".to_string(),
        })?;

    let parent_canonical = if parent.exists() {
        parent
            .canonicalize()
            .map_err(|e| MdrefError::PathValidation {
                path: parent.to_path_buf(),
                details: format!("cannot canonicalize parent directory: {e}"),
            })?
    } else {
        parent.to_path_buf()
    };

    let filename = destination
        .file_name()
        .ok_or_else(|| MdrefError::PathValidation {
            path: destination.to_path_buf(),
            details: "destination path has no filename".to_string(),
        })?;

    Ok(parent_canonical.join(filename))
}

/// Validate that the move operation is valid: source exists, destination doesn't collide, etc.
/// Returns `(resolved_dest, source_canonical, dest_canonical)`.
pub(super) fn validate_move_paths(
    source: &Path,
    destination: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    if !source.exists() {
        return Err(MdrefError::PathValidation {
            path: source.to_path_buf(),
            details: "source path does not exist".to_string(),
        });
    }

    let source_canonical = source
        .canonicalize()
        .map_err(|e| MdrefError::PathValidation {
            path: source.to_path_buf(),
            details: format!("cannot canonicalize source path: {e}"),
        })?;

    let resolved_dest = resolve_destination(source, destination)?;
    let dest_canonical = canonicalize_destination(&resolved_dest)?;

    if source_canonical == dest_canonical {
        return Err(MdrefError::PathValidation {
            path: source.to_path_buf(),
            details: "source and destination resolve to the same file".to_string(),
        });
    }

    if resolved_dest.exists() {
        return Err(MdrefError::PathValidation {
            path: resolved_dest.clone(),
            details: "destination path already exists".to_string(),
        });
    }

    if source_canonical.is_dir() && dest_canonical.starts_with(&source_canonical) {
        return Err(MdrefError::PathValidation {
            path: source.to_path_buf(),
            details: "cannot move directory into itself or one of its subdirectories".to_string(),
        });
    }

    Ok((resolved_dest, source_canonical, dest_canonical))
}
