//! Case-only rename support (e.g. `Foo.md` → `foo.md` on the same parent).
//!
//! On case-insensitive filesystems (macOS default, Windows) a case-only rename
//! cannot be detected by comparing canonicalized paths — both sides canonicalize
//! to the same inode. These helpers detect that situation explicitly and plan
//! link rewrites while preserving the new filename case.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::{
    plan::{LineCache, ReplacementPlan, build_replacement, split_link_and_anchor},
    validate::{canonicalize_destination, resolve_destination},
};
use crate::{MdrefError, Reference, Result, core::pathdiff::diff_paths};

/// Detect a case-only rename within the same parent directory.
///
/// Returns `Some(resolved_dest)` when `source` and `destination` canonicalize to
/// the same inode but the filename differs only in ASCII case; otherwise `None`.
pub(super) fn resolve_case_only_destination(
    source: &Path,
    destination: &Path,
) -> Result<Option<PathBuf>> {
    if !source.exists() {
        return Ok(None);
    }

    let source_canonical = source
        .canonicalize()
        .map_err(|e| MdrefError::PathValidation {
            path: source.to_path_buf(),
            details: format!("cannot canonicalize source path: {e}"),
        })?;
    let resolved_dest = resolve_destination(source, destination)?;
    let dest_canonical = canonicalize_destination(&resolved_dest)?;
    let same_parent = source.parent() == resolved_dest.parent();
    let case_only_name_change = source
        .file_name()
        .zip(resolved_dest.file_name())
        .map(|(source_name, dest_name)| {
            let source_name = source_name.to_string_lossy();
            let dest_name = dest_name.to_string_lossy();
            source_name != dest_name && source_name.eq_ignore_ascii_case(&dest_name)
        })
        .unwrap_or(false);

    if source_canonical == dest_canonical && same_parent && case_only_name_change {
        Ok(Some(resolved_dest))
    } else {
        Ok(None)
    }
}

/// Compute the relative path from `from` to `to` while preserving `to`'s
/// filename case.
///
/// `Path::canonicalize` would collapse the filename case on case-insensitive
/// filesystems, which defeats the whole point of a case-only rename.
fn relative_path_preserving_filename_case(from: &Path, to: &Path) -> Result<PathBuf> {
    let from_parent = from.parent().ok_or_else(|| MdrefError::PathValidation {
        path: from.to_path_buf(),
        details: "no parent directory".to_string(),
    })?;
    let from_resolved = if from_parent.exists() {
        from_parent.canonicalize()?
    } else {
        crate::core::util::resolve_parent(from_parent)?
    };

    let to_parent = to.parent().ok_or_else(|| MdrefError::PathValidation {
        path: to.to_path_buf(),
        details: "no parent directory".to_string(),
    })?;
    let to_parent_resolved = if to_parent.exists() {
        to_parent.canonicalize()?
    } else {
        crate::core::util::resolve_parent(to_parent)?
    };
    let filename = to.file_name().ok_or_else(|| MdrefError::PathValidation {
        path: to.to_path_buf(),
        details: "no file name".to_string(),
    })?;

    Ok(diff_paths(to_parent_resolved.join(filename), from_resolved).unwrap_or_default())
}

/// Plan replacements for external references when the move is a case-only rename.
pub(super) fn plan_case_only_external_replacements(
    references: &[Reference],
    resolved_dest: &Path,
) -> Result<ReplacementPlan> {
    let mut replacements_by_file: ReplacementPlan = HashMap::new();
    let mut line_cache = LineCache::new();

    for reference in references {
        let (_link_path_only, anchor) = split_link_and_anchor(&reference.link_text);
        let new_link_path = relative_path_preserving_filename_case(&reference.path, resolved_dest)?;

        let new_link_with_anchor = match anchor {
            Some(a) => format!("{}#{}", new_link_path.display(), a),
            None => new_link_path.display().to_string(),
        };

        replacements_by_file
            .entry(reference.path.clone())
            .or_default()
            .push(build_replacement(
                reference,
                &new_link_with_anchor,
                &mut line_cache,
            )?);
    }

    Ok(replacements_by_file)
}
