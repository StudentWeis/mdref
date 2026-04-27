//! Filesystem-mutating phase of a move.
//!
//! Everything in this module either writes files or orchestrates rollback.
//! Callers are expected to have already built a [`super::plan::ReplacementPlan`]
//! and a `MoveTransaction` snapshotting every file that will be touched.
//!
//! The module covers three sub-concerns:
//!
//! - rollback orchestration: [`execute_with_rollback`]
//! - regular-file rename with cross-device fallback: [`RegularFileMoveMethod`],
//!   [`try_rename_regular_file`] (and the injectable variant used in tests)
//! - in-place file rewriting that preserves original line endings:
//!   [`apply_replacements`] plus the `LineEnding` helpers

use std::{fs, path::Path};

use crate::{
    MdrefError, Result,
    core::model::{LinkReplacement, MoveTransaction},
};

// ============= Rollback orchestration =============

/// Execute a fallible closure within a transaction context.
/// If the closure returns an error, the transaction is rolled back automatically.
pub(super) fn execute_with_rollback<F>(transaction: &MoveTransaction, operation: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    match operation() {
        Ok(()) => Ok(()),
        Err(original_error) => {
            let rollback_errors = transaction.rollback();
            if rollback_errors.is_empty() {
                Err(original_error)
            } else {
                Err(MdrefError::RollbackFailed {
                    original_error: original_error.to_string(),
                    rollback_errors,
                })
            }
        }
    }
}

// ============= Regular-file rename with fallback =============

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RegularFileMoveMethod {
    Renamed,
    CopyAndDelete,
}

pub(super) fn try_rename_regular_file(
    source: &Path,
    dest: &Path,
) -> std::io::Result<RegularFileMoveMethod> {
    try_rename_regular_file_with(source, dest, |from, to| fs::rename(from, to))
}

pub(super) fn try_rename_regular_file_with<F>(
    source: &Path,
    dest: &Path,
    rename: F,
) -> std::io::Result<RegularFileMoveMethod>
where
    F: FnOnce(&Path, &Path) -> std::io::Result<()>,
{
    match rename(source, dest) {
        Ok(()) => Ok(RegularFileMoveMethod::Renamed),
        Err(error) if error.kind() == std::io::ErrorKind::CrossesDevices => {
            Ok(RegularFileMoveMethod::CopyAndDelete)
        }
        Err(error) => Err(error),
    }
}

// ============= File rewriting with line-ending preservation =============

#[derive(Clone, Copy)]
enum LineEnding {
    None,
    Lf,
    CrLf,
}

impl LineEnding {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
        }
    }
}

fn split_lines_preserving_endings(content: &str) -> Vec<(String, LineEnding)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = content.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'\n' {
            let is_crlf = index > 0 && bytes[index - 1] == b'\r';
            let line_end = if is_crlf { index - 1 } else { index };
            let line = content[start..line_end].to_string();
            let ending = if is_crlf {
                LineEnding::CrLf
            } else {
                LineEnding::Lf
            };
            lines.push((line, ending));
            start = index + 1;
        }
        index += 1;
    }

    if start < content.len() {
        lines.push((content[start..].to_string(), LineEnding::None));
    }

    lines
}

/// Apply all pending replacements to a single file in one read-write cycle.
/// Replacements are sorted in reverse order (by line desc, then column desc) so that
/// earlier replacements do not shift the positions of later ones.
pub(super) fn apply_replacements(file_path: &Path, replacements: &[LinkReplacement]) -> Result<()> {
    let content = fs::read_to_string(file_path).map_err(|e| MdrefError::IoRead {
        path: file_path.to_path_buf(),
        source: e,
    })?;
    let mut lines = split_lines_preserving_endings(&content);

    // Sort replacements in reverse order (bottom-right to top-left) so that
    // replacing one link does not invalidate the positions of subsequent ones.
    let mut sorted_indices: Vec<usize> = (0..replacements.len()).collect();
    sorted_indices.sort_by(|&a, &b| {
        replacements[b]
            .line
            .cmp(&replacements[a].line)
            .then_with(|| replacements[b].column.cmp(&replacements[a].column))
    });

    for &idx in &sorted_indices {
        let replacement = &replacements[idx];

        if replacement.line > lines.len() {
            return Err(MdrefError::InvalidLineReference {
                path: file_path.to_path_buf(),
                line: replacement.line,
                details: format!("line number out of range (file has {} lines)", lines.len()),
            });
        }

        let line = &lines[replacement.line - 1].0;
        let col = replacement.column.saturating_sub(1); // Convert to 0-based index

        // Search for the old_pattern starting from the column position.
        // This ensures we replace the correct occurrence when multiple identical links exist.
        if let Some(pos) = line[col..].find(&replacement.old_pattern) {
            let actual_pos = col + pos;
            let end_pos = actual_pos + replacement.old_pattern.len();
            let new_line = format!(
                "{}{}{}",
                &line[..actual_pos],
                replacement.new_pattern,
                &line[end_pos..]
            );
            lines[replacement.line - 1].0 = new_line;
        } else {
            return Err(MdrefError::PathValidation {
                path: file_path.to_path_buf(),
                details: format!(
                    "could not find link '{}' in line {}",
                    replacement.old_pattern, replacement.line
                ),
            });
        }
    }

    let new_content = lines
        .into_iter()
        .map(|(line, ending)| format!("{line}{}", ending.as_str()))
        .collect::<String>();
    fs::write(file_path, new_content).map_err(|e| MdrefError::IoWrite {
        path: file_path.to_path_buf(),
        source: e,
    })?;

    Ok(())
}
