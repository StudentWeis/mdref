//! Preview / dry-run rendering.
//!
//! Pure transformations from a [`ReplacementPlan`] into user-facing shapes:
//!
//! - [`build_move_preview`]: assemble the structured [`MovePreview`] returned by
//!   the public `preview_move` API.
//! - [`print_dry_run_report`]: human-readable stdout report used when the CLI
//!   runs with `--dry-run`.

use std::path::Path;

use super::plan::ReplacementPlan;
use crate::core::model::{MoveChange, MoveChangeKind, MovePreview};

pub(super) fn build_move_preview(
    source: &Path,
    destination: &Path,
    replacements_by_file: ReplacementPlan,
) -> MovePreview {
    let mut changes = replacements_by_file
        .into_iter()
        .map(|(path, mut replacements)| {
            replacements.sort_by(|left, right| {
                left.line
                    .cmp(&right.line)
                    .then(left.column.cmp(&right.column))
                    .then(left.old_pattern.cmp(&right.old_pattern))
                    .then(left.new_pattern.cmp(&right.new_pattern))
            });

            let kind = if path == destination {
                MoveChangeKind::MovedFileUpdate
            } else {
                MoveChangeKind::ReferenceUpdate
            };

            MoveChange {
                path,
                kind,
                replacements,
            }
        })
        .collect::<Vec<_>>();

    changes.sort_by(|left, right| left.path.cmp(&right.path));

    MovePreview {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        changes,
    }
}

/// Print a human-readable report of all changes that would be made during a move operation.
pub(super) fn print_dry_run_report(preview: &MovePreview) {
    println!(
        "[dry-run] Would move: {} -> {}",
        preview.source.display(),
        preview.destination.display()
    );

    if preview.changes.is_empty() {
        println!("[dry-run] No references to update.");
        return;
    }

    for change in &preview.changes {
        let label = match change.kind {
            MoveChangeKind::MovedFileUpdate => "Would update links in moved file",
            MoveChangeKind::ReferenceUpdate => "Would update reference in",
        };
        println!("[dry-run] {} {}:", label, change.path.display());
        for replacement in &change.replacements {
            println!(
                "  Line {}: {} -> {}",
                replacement.line, replacement.old_pattern, replacement.new_pattern
            );
        }
    }
}
