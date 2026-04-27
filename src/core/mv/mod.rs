//! Move a Markdown file or directory and atomically update all references.
//!
//! The heavy lifting is split across responsibility-oriented submodules:
//!
//! - [`validate`]: path existence / collision / self-move checks
//! - [`case_only`]: detect and plan case-only renames on case-insensitive FSes
//! - [`plan`]: turn a source→destination pair into a per-file replacement plan
//! - [`apply`]: mutate the filesystem (rename / copy+delete, rewrite files,
//!   execute under a rollback-protected transaction)
//! - [`preview`]: render dry-run reports and the structured [`MovePreview`]
//!
//! This top-level file keeps only the public API (`mv`, `preview_move`) and the
//! three orchestration routines for regular files, case-only renames, and
//! directory moves.

mod apply;
mod case_only;
mod plan;
mod preview;
mod validate;

use std::{collections::HashMap, fs, path::Path};

use self::{
    apply::{
        RegularFileMoveMethod, apply_replacements, execute_with_rollback, try_rename_regular_file,
    },
    case_only::{plan_case_only_external_replacements, resolve_case_only_destination},
    plan::{
        add_destination_replacements, move_source_replacements_to_destination,
        plan_directory_replacements, plan_external_replacements, plan_internal_replacements,
    },
    preview::{build_move_preview, print_dry_run_report},
    validate::validate_move_paths,
};
// Re-export the structured preview shape so callers can match on it.
pub use crate::core::model::MovePreview;
use crate::{
    Result,
    core::{find::find_references, model::MoveTransaction, progress::ProgressReporter},
};

// ============= Public API =============

/// Move a Markdown file or directory and atomically update all references across the project.
///
/// This function finds all references to the source file and updates them to point to the
/// new location. It also updates links within the moved file itself to ensure they remain valid.
///
/// **Atomicity guarantee**: all filesystem mutations are tracked in a transaction. If any step
/// fails, all changes are rolled back — modified files are restored to their original content,
/// the copied destination is removed, and the deleted source is recovered.
///
/// When `dry_run` is `true`, no files are created, moved, or modified. Instead, the function
/// prints all changes that *would* be made, allowing the user to preview the operation.
///
/// If the destination path is an existing directory, the source file will be moved into that
/// directory with its original filename preserved.
///
/// # Progress
///
/// Callers pass a [`ProgressReporter`] trait object to receive scanning progress.
/// Pass [`crate::NoopProgress`] (as `&NoopProgress`) when progress updates are not needed.
pub fn mv<P, B, D>(
    source: P,
    dest: B,
    root: D,
    dry_run: bool,
    progress: &dyn ProgressReporter,
) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let source = source.as_ref();
    let dest = dest.as_ref();
    let root = root.as_ref();

    if source.is_dir() {
        return mv_directory(source, dest, root, dry_run, progress);
    }

    mv_regular_file(source, dest, root, dry_run, progress)
}

/// Preview a Markdown move without mutating the filesystem.
///
/// The returned preview contains the resolved destination path and all link
/// replacements that would be applied by the move.
///
/// # Progress
///
/// Callers pass a [`ProgressReporter`] trait object to receive scanning progress.
/// Pass [`crate::NoopProgress`] (as `&NoopProgress`) when progress updates are not needed.
pub fn preview_move<P, B, D>(
    source: P,
    dest: B,
    root: D,
    progress: &dyn ProgressReporter,
) -> Result<MovePreview>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let source = source.as_ref();
    let dest = dest.as_ref();
    let root = root.as_ref();

    if source.is_dir() {
        return preview_directory_move(source, dest, root, progress);
    }

    preview_regular_file_move(source, dest, root, progress)
}

// ============= Orchestration: preview =============

fn preview_regular_file_move(
    source: &Path,
    dest: &Path,
    root: &Path,
    progress: &dyn ProgressReporter,
) -> Result<MovePreview> {
    if let Some(case_only_dest) = resolve_case_only_destination(source, dest)? {
        return preview_case_only_file_move(source, &case_only_dest, root, progress);
    }

    let (resolved_dest, _source_canonical, _dest_canonical) =
        match validate_move_paths(source, dest) {
            Ok(paths) => paths,
            Err(e) => {
                if e.to_string().contains("resolve to the same file") {
                    return Ok(build_move_preview(source, source, HashMap::new()));
                }
                return Err(e);
            }
        };

    progress.set_message("Scanning references...");
    let references = find_references(source, root, progress)?;
    let mut replacements_by_file = plan_external_replacements(&references, &resolved_dest)?;
    replacements_by_file.remove(source);
    let internal_replacements = plan_internal_replacements(source, source, &resolved_dest)?;
    add_destination_replacements(
        &mut replacements_by_file,
        &resolved_dest,
        internal_replacements,
    );

    Ok(build_move_preview(
        source,
        &resolved_dest,
        replacements_by_file,
    ))
}

fn preview_case_only_file_move(
    source: &Path,
    resolved_dest: &Path,
    root: &Path,
    progress: &dyn ProgressReporter,
) -> Result<MovePreview> {
    progress.set_message("Scanning references...");
    let references = find_references(source, root, progress)?;
    let mut replacements_by_file =
        plan_case_only_external_replacements(&references, resolved_dest)?;
    move_source_replacements_to_destination(&mut replacements_by_file, source, resolved_dest);

    Ok(build_move_preview(
        source,
        resolved_dest,
        replacements_by_file,
    ))
}

fn preview_directory_move(
    source_dir: &Path,
    new_path: &Path,
    root: &Path,
    progress: &dyn ProgressReporter,
) -> Result<MovePreview> {
    let (resolved_dest, source_canonical, dest_canonical) =
        match validate_move_paths(source_dir, new_path) {
            Ok(paths) => paths,
            Err(e) => {
                if e.to_string().contains("resolve to the same file") {
                    return Ok(build_move_preview(source_dir, source_dir, HashMap::new()));
                }
                return Err(e);
            }
        };

    let (replacements_by_file, _snapshot_paths) = plan_directory_replacements(
        source_dir,
        &source_canonical,
        &dest_canonical,
        root,
        progress,
    )?;

    Ok(build_move_preview(
        source_dir,
        &resolved_dest,
        replacements_by_file,
    ))
}

// ============= Orchestration: mutating move =============

fn mv_regular_file(
    source: &Path,
    dest: &Path,
    root: &Path,
    dry_run: bool,
    progress: &dyn ProgressReporter,
) -> Result<()> {
    if let Some(case_only_dest) = resolve_case_only_destination(source, dest)? {
        return mv_case_only_file(source, &case_only_dest, root, dry_run, progress);
    }

    let (resolved_dest, _source_canonical, _dest_canonical) =
        match validate_move_paths(source, dest) {
            Ok(paths) => paths,
            Err(e) => {
                // Special case: source == destination is a no-op, not an error.
                if e.to_string().contains("resolve to the same file") {
                    return Ok(());
                }
                return Err(e);
            }
        };

    // Phase 1: Plan — pure computation, no side effects.
    progress.set_message("Scanning references...");
    let references = find_references(source, root, progress)?;
    let mut replacements_by_file = plan_external_replacements(&references, &resolved_dest)?;
    replacements_by_file.remove(source);
    let internal_replacements = plan_internal_replacements(source, source, &resolved_dest)?;

    if dry_run {
        add_destination_replacements(
            &mut replacements_by_file,
            &resolved_dest,
            internal_replacements,
        );
        let preview = build_move_preview(source, &resolved_dest, replacements_by_file);
        print_dry_run_report(&preview);
        return Ok(());
    }

    // Phase 2: Execute — all mutations are tracked for rollback.
    let mut transaction = MoveTransaction::new(source.to_path_buf(), resolved_dest.clone());

    // Snapshot all files that will be modified before touching anything.
    for file_path in replacements_by_file.keys() {
        transaction.snapshot_file(file_path)?;
    }

    if !internal_replacements.is_empty() && !replacements_by_file.contains_key(source) {
        transaction.snapshot_file(source)?;
    }

    // Ensure the parent directory of the destination exists.
    if let Some(parent) = resolved_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let move_method = try_rename_regular_file(source, &resolved_dest)?;
    match move_method {
        RegularFileMoveMethod::Renamed => {
            transaction.mark_renamed();
            add_destination_replacements(
                &mut replacements_by_file,
                &resolved_dest,
                internal_replacements,
            );
        }
        RegularFileMoveMethod::CopyAndDelete => {
            fs::copy(source, &resolved_dest)?;
            transaction.mark_copied();

            if !internal_replacements.is_empty() {
                transaction.snapshot_file(&resolved_dest)?;
                add_destination_replacements(
                    &mut replacements_by_file,
                    &resolved_dest,
                    internal_replacements,
                );
            }
        }
    }

    // Apply all replacements within a rollback-protected context.
    execute_with_rollback(&transaction, || {
        for (file_path, replacements) in &replacements_by_file {
            apply_replacements(file_path, replacements)?;
        }
        Ok(())
    })?;

    if move_method == RegularFileMoveMethod::CopyAndDelete {
        if let Err(original_error) = fs::remove_file(source) {
            let rollback_errors = transaction.rollback();
            return if rollback_errors.is_empty() {
                Err(original_error.into())
            } else {
                Err(crate::MdrefError::RollbackFailed {
                    original_error: original_error.to_string(),
                    rollback_errors,
                })
            };
        }

        transaction.mark_source_removed();
    }

    Ok(())
}

fn mv_case_only_file(
    source: &Path,
    resolved_dest: &Path,
    root: &Path,
    dry_run: bool,
    progress: &dyn ProgressReporter,
) -> Result<()> {
    progress.set_message("Scanning references...");
    let references = find_references(source, root, progress)?;
    let mut replacements_by_file =
        plan_case_only_external_replacements(&references, resolved_dest)?;
    move_source_replacements_to_destination(&mut replacements_by_file, source, resolved_dest);

    if dry_run {
        let preview = build_move_preview(source, resolved_dest, replacements_by_file);
        print_dry_run_report(&preview);
        return Ok(());
    }

    let mut transaction = MoveTransaction::new(source.to_path_buf(), resolved_dest.to_path_buf());
    if replacements_by_file.contains_key(resolved_dest) {
        transaction.snapshot_file(source)?;
    }
    for file_path in replacements_by_file
        .keys()
        .filter(|path| *path != resolved_dest)
    {
        transaction.snapshot_file(file_path)?;
    }

    fs::rename(source, resolved_dest)?;
    transaction.mark_renamed();

    execute_with_rollback(&transaction, || {
        for (file_path, replacements) in &replacements_by_file {
            apply_replacements(file_path, replacements)?;
        }
        Ok(())
    })
}

fn mv_directory(
    source_dir: &Path,
    new_path: &Path,
    root: &Path,
    dry_run: bool,
    progress: &dyn ProgressReporter,
) -> Result<()> {
    let (resolved_dest, source_canonical, dest_canonical) =
        match validate_move_paths(source_dir, new_path) {
            Ok(paths) => paths,
            Err(e) => {
                if e.to_string().contains("resolve to the same file") {
                    return Ok(());
                }
                return Err(e);
            }
        };

    let (replacements_by_file, snapshot_paths) = plan_directory_replacements(
        source_dir,
        &source_canonical,
        &dest_canonical,
        root,
        progress,
    )?;

    if dry_run {
        let preview = build_move_preview(source_dir, &resolved_dest, replacements_by_file);
        print_dry_run_report(&preview);
        return Ok(());
    }

    let mut transaction = MoveTransaction::new(source_dir.to_path_buf(), resolved_dest.clone());
    for snapshot_path in snapshot_paths {
        transaction.snapshot_file(&snapshot_path)?;
    }

    if let Some(parent) = resolved_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source_dir, &resolved_dest)?;
    transaction.mark_renamed();

    execute_with_rollback(&transaction, || {
        for (file_path, replacements) in &replacements_by_file {
            apply_replacements(file_path, replacements)?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{
        apply::{
            RegularFileMoveMethod, apply_replacements, execute_with_rollback,
            try_rename_regular_file_with,
        },
        plan::{LineCache, build_link_replacement, find_reference_definition_url_span},
        *,
    };
    use crate::{
        MdrefError, Reference,
        core::{model::LinkReplacement, util::relative_path},
        test_utils::write_file,
    };

    // ============= apply_replacements tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_single_link_rewrites_target_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](old.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](new.md)"));
        assert!(!content.contains("](old.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_preserves_other_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(
            file_path.to_str().unwrap(),
            "# Title\n\nSome text [Link](old.md) more text.\n\nAnother paragraph.",
        );

        let replacements = vec![LinkReplacement {
            line: 3,
            column: 11,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("# Title"));
        assert!(content.contains("Some text [Link](new.md) more text."));
        assert!(content.contains("Another paragraph."));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_crlf_input_preserves_crlf_line_endings() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(
            file_path.to_str().unwrap(),
            "# Title\r\n\r\nSee [Link](old.md)\r\n",
        );

        let replacements = vec![LinkReplacement {
            line: 3,
            column: 5,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "# Title\r\n\r\nSee [Link](new.md)\r\n");
        assert!(!content.contains('\n') || content.contains("\r\n"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_line_out_of_range_returns_invalid_line_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "Single line");

        let replacements = vec![LinkReplacement {
            line: 999,
            column: 1,
            old_pattern: "](link.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        let result = apply_replacements(&file_path, &replacements);
        match result {
            Err(MdrefError::InvalidLineReference { path, line, .. }) => {
                assert_eq!(line, 999);
                assert!(path.ends_with("doc.md"));
            }
            other => panic!("expected invalid line error, got {other:?}"),
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_with_subdirectory_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](sub/old.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](sub/old.md)".to_string(),
            new_pattern: "](other/new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](other/new.md)"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_only_replaces_target_link() {
        // Verify that when two identical links exist on the same line,
        // only the one at the specified column is replaced.
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[A](doc.md) and [B](doc.md)");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](doc.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(
            content.contains("[A](new.md)"),
            "Expected [A](new.md) in content: {}",
            content
        );
        assert!(
            content.contains("[B](doc.md)"),
            "Bug: [B](doc.md) was incorrectly modified. Content: {}",
            content
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_multiple_in_same_file() {
        // Verify that multiple replacements in the same file are applied correctly
        // in a single read-write cycle.
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(
            file_path.to_str().unwrap(),
            "[Link1](old.md)\n\n[Link2](old.md)\n\n[Link3](old.md)",
        );

        let replacements = vec![
            LinkReplacement {
                line: 1,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
            LinkReplacement {
                line: 3,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
            LinkReplacement {
                line: 5,
                column: 1,
                old_pattern: "](old.md)".to_string(),
                new_pattern: "](new.md)".to_string(),
            },
        ];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(!content.contains("](old.md)"));
        assert_eq!(content.matches("](new.md)").count(), 3);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_apply_replacements_preserves_trailing_newline() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("doc.md");
        write_file(file_path.to_str().unwrap(), "[Link](old.md)\n");

        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: "](new.md)".to_string(),
        }];

        apply_replacements(&file_path, &replacements).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("](new.md)"));
        assert!(content.ends_with('\n'), "Trailing newline was lost");
    }

    // ============= update via relative_path + apply_replacements tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_same_directory() {
        let temp_dir = TempDir::new().unwrap();
        let ref_file = temp_dir.path().join("ref.md");
        let new_target = temp_dir.path().join("new_target.md");
        write_file(ref_file.to_str().unwrap(), "[Link](old_target.md)");
        write_file(new_target.to_str().unwrap(), "");

        let new_link_path = relative_path(&ref_file, &new_target).unwrap();
        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old_target.md)".to_string(),
            new_pattern: format!("]({})", new_link_path.display()),
        }];

        apply_replacements(&ref_file, &replacements).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("new_target.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_update_reference_cross_directory() {
        let temp_dir = TempDir::new().unwrap();
        let ref_file = temp_dir.path().join("ref.md");
        let new_target = temp_dir.path().join("sub").join("new_target.md");
        write_file(ref_file.to_str().unwrap(), "[Link](old.md)");
        write_file(new_target.to_str().unwrap(), "");

        let new_link_path = relative_path(&ref_file, &new_target).unwrap();
        let replacements = vec![LinkReplacement {
            line: 1,
            column: 1,
            old_pattern: "](old.md)".to_string(),
            new_pattern: format!("]({})", new_link_path.display()),
        }];

        apply_replacements(&ref_file, &replacements).unwrap();

        let content = fs::read_to_string(&ref_file).unwrap();
        assert!(content.contains("sub/new_target.md"));
    }

    // ============= build_link_replacement with external URL =============

    // ============= build_link_replacement with anchored internal links =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_preserves_anchor() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let other = temp_dir.path().join("other.md");
        let target = temp_dir.path().join("sub").join("target.md");
        write_file(source.to_str().unwrap(), "[Details](other.md#details)");
        write_file(other.to_str().unwrap(), "# Other");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "other.md#details".to_string());

        let mut line_cache = LineCache::new();
        let result = build_link_replacement(&reference, &source, &target, &mut line_cache).unwrap();
        assert!(
            result.is_some(),
            "Should produce a replacement for anchored link"
        );

        let replacement = result.unwrap();
        assert!(
            replacement.new_pattern.contains("#details"),
            "Anchor should be preserved in new pattern. Got: {}",
            replacement.new_pattern
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_broken_link() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[Broken](nonexistent.md)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "nonexistent.md".to_string());

        // Should return Ok(None) for broken links, not Err
        let mut line_cache = LineCache::new();
        let result = build_link_replacement(&reference, &source, &target, &mut line_cache);
        assert!(
            result.is_ok(),
            "build_link_replacement should not error on broken links: {:?}",
            result.err()
        );
        assert!(
            result.unwrap().is_none(),
            "Broken links should be skipped (return None)"
        );
    }

    // ============= build_link_replacement with pure anchor links =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_pure_anchor_link() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("sub").join("target.md");
        write_file(source.to_str().unwrap(), "[Section](#section)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "#section".to_string());

        // Pure anchor links are internal to the file and should not be rewritten
        let mut line_cache = LineCache::new();
        let result = build_link_replacement(&reference, &source, &target, &mut line_cache).unwrap();
        assert!(
            result.is_none(),
            "Pure anchor link (#section) should be skipped, but got: {:?}",
            result.map(|r| r.new_pattern)
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_pure_anchor_with_complex_fragment() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[TOC](#table-of-contents)");
        write_file(target.to_str().unwrap(), "");

        let reference = Reference::new(target.clone(), 1, 1, "#table-of-contents".to_string());

        let mut line_cache = LineCache::new();
        let result = build_link_replacement(&reference, &source, &target, &mut line_cache).unwrap();
        assert!(
            result.is_none(),
            "Pure anchor link (#table-of-contents) should be skipped"
        );
    }

    // ============= build_link_replacement with external URL =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_build_link_replacement_skips_external_url() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let target = temp_dir.path().join("target.md");
        write_file(source.to_str().unwrap(), "[Google](https://google.com)");
        write_file(target.to_str().unwrap(), "[Google](https://google.com)");

        let reference = Reference::new(target.clone(), 1, 1, "https://google.com".to_string());

        // Should return None — external URL is skipped
        let mut line_cache = LineCache::new();
        let result = build_link_replacement(&reference, &source, &target, &mut line_cache).unwrap();
        assert!(result.is_none());

        // Content should remain unchanged
        let content = fs::read_to_string(&target).unwrap();
        assert!(content.contains("https://google.com"));
    }

    #[test]
    fn test_find_reference_definition_url_span_preserves_angle_brackets_and_spacing() {
        let line = "[ref]:    <target.md> \"Title\"";
        let span = find_reference_definition_url_span(line).unwrap();

        assert_eq!(&line[span.0..span.1], "target.md");
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_execute_with_rollback_returns_rollback_failed_when_snapshot_restore_fails() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_file = temp_dir.path().join("refs").join("index.md");
        write_file(snapshot_file.to_str().unwrap(), "[Doc](source.md)");

        let mut transaction = MoveTransaction::new(
            temp_dir.path().join("source.md"),
            temp_dir.path().join("moved").join("source.md"),
        );
        transaction.snapshot_file(&snapshot_file).unwrap();

        fs::remove_dir_all(snapshot_file.parent().unwrap()).unwrap();

        let result = execute_with_rollback(&transaction, || {
            Err(MdrefError::PathValidation {
                path: PathBuf::from("simulated"),
                details: "simulated move failure".to_string(),
            })
        });

        match result {
            Err(MdrefError::RollbackFailed {
                original_error,
                rollback_errors,
            }) => {
                assert_eq!(
                    original_error,
                    "Path error for 'simulated': simulated move failure"
                );
                assert_eq!(rollback_errors.len(), 1);
                assert!(rollback_errors[0].contains("Failed to restore"));
                assert!(rollback_errors[0].contains("index.md"));
            }
            other => panic!("expected rollback failed error, got {other:?}"),
        }
    }

    #[test]
    fn test_try_rename_regular_file_with_requests_copy_fallback_for_cross_device_error() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let destination = temp_dir.path().join("dest.md");
        write_file(source.to_str().unwrap(), "# Source");

        let result = try_rename_regular_file_with(&source, &destination, |_, _| {
            Err(std::io::Error::from(std::io::ErrorKind::CrossesDevices))
        })
        .unwrap();

        assert_eq!(result, RegularFileMoveMethod::CopyAndDelete);
        assert!(source.exists());
        assert!(!destination.exists());
    }

    #[test]
    fn test_try_rename_regular_file_with_propagates_non_cross_device_error() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source.md");
        let destination = temp_dir.path().join("dest.md");
        write_file(source.to_str().unwrap(), "# Source");

        let result = try_rename_regular_file_with(&source, &destination, |_, _| {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        });

        assert!(matches!(
            result,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied
        ));
        assert!(source.exists());
        assert!(!destination.exists());
    }
}
