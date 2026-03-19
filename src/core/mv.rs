use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use pathdiff::diff_paths;
use walkdir::WalkDir;

use super::{
    model::{LinkReplacement, MoveTransaction},
    util::{is_external_url, relative_path, url_decode_link},
};
use crate::{LinkType, MdrefError, Reference, Result, find_links, find_references};

type ReplacementPlan = HashMap<PathBuf, Vec<LinkReplacement>>;
type SnapshotPaths = Vec<PathBuf>;
type LineCache = HashMap<PathBuf, Vec<String>>;

// LinkReplacement and MoveTransaction are now defined in the model module

/// Execute a fallible closure within a transaction context.
/// If the closure returns an error, the transaction is rolled back automatically.
fn execute_with_rollback<F>(transaction: &MoveTransaction, operation: F) -> Result<()>
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

// ============= Path resolution =============

/// Resolve the destination path, handling the case where the destination is an existing directory.
fn resolve_destination(source: &Path, destination: &Path) -> Result<PathBuf> {
    if destination.is_dir() {
        let filename = source.file_name().ok_or_else(|| {
            MdrefError::Path(format!("Source path has no filename: {}", source.display()))
        })?;
        Ok(destination.join(filename))
    } else {
        Ok(destination.to_path_buf())
    }
}

/// Canonicalize a destination path, handling the case where it doesn't exist yet.
fn canonicalize_destination(destination: &Path) -> Result<PathBuf> {
    if destination.exists() {
        return destination.canonicalize().map_err(|e| {
            MdrefError::Path(format!(
                "Cannot canonicalize destination path '{}': {}",
                destination.display(),
                e
            ))
        });
    }

    let parent = destination.parent().ok_or_else(|| {
        MdrefError::Path(format!(
            "Destination path has no parent directory: {}",
            destination.display()
        ))
    })?;

    let parent_canonical = if parent.exists() {
        parent.canonicalize().map_err(|e| {
            MdrefError::Path(format!(
                "Cannot canonicalize parent directory '{}': {}",
                parent.display(),
                e
            ))
        })?
    } else {
        parent.to_path_buf()
    };

    let filename = destination.file_name().ok_or_else(|| {
        MdrefError::Path(format!(
            "Destination path has no filename: {}",
            destination.display()
        ))
    })?;

    Ok(parent_canonical.join(filename))
}

/// Validate that the move operation is valid: source exists, destination doesn't collide, etc.
/// Returns `(resolved_dest, source_canonical, dest_canonical)`.
fn validate_move_paths(source: &Path, destination: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
    if !source.exists() {
        return Err(MdrefError::Path(format!(
            "Source path does not exist: {}",
            source.display()
        )));
    }

    let source_canonical = source.canonicalize().map_err(|e| {
        MdrefError::Path(format!(
            "Cannot canonicalize source path '{}': {}",
            source.display(),
            e
        ))
    })?;

    let resolved_dest = resolve_destination(source, destination)?;
    let dest_canonical = canonicalize_destination(&resolved_dest)?;

    if source_canonical == dest_canonical {
        return Err(MdrefError::Path(
            "Source and destination resolve to the same file".to_string(),
        ));
    }

    if resolved_dest.exists() {
        return Err(MdrefError::Path(format!(
            "Destination path already exists: {}",
            resolved_dest.display()
        )));
    }

    if source_canonical.is_dir() && dest_canonical.starts_with(&source_canonical) {
        return Err(MdrefError::Path(format!(
            "Cannot move directory '{}' into itself or one of its subdirectories",
            source.display()
        )));
    }

    Ok((resolved_dest, source_canonical, dest_canonical))
}

fn resolve_case_only_destination(source: &Path, destination: &Path) -> Result<Option<PathBuf>> {
    if !source.exists() {
        return Ok(None);
    }

    let source_canonical = source.canonicalize().map_err(|e| {
        MdrefError::Path(format!(
            "Cannot canonicalize source path '{}': {}",
            source.display(),
            e
        ))
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

// ============= Replacement planning =============

/// Collect all link replacements needed for external references (other files pointing to the moved file).
fn plan_external_replacements(
    references: &[Reference],
    resolved_dest: &Path,
) -> Result<ReplacementPlan> {
    let mut replacements_by_file: ReplacementPlan = HashMap::new();
    let mut line_cache = LineCache::new();

    for reference in references {
        let (_link_path_only, anchor) = split_link_and_anchor(&reference.link_text);
        let new_link_path = relative_path(&reference.path, resolved_dest)?;

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

fn relative_path_preserving_filename_case(from: &Path, to: &Path) -> Result<PathBuf> {
    let from_parent = from
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let from_resolved = if from_parent.exists() {
        from_parent.canonicalize()?
    } else {
        super::util::resolve_parent(from_parent)?
    };

    let to_parent = to
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
    let to_parent_resolved = if to_parent.exists() {
        to_parent.canonicalize()?
    } else {
        super::util::resolve_parent(to_parent)?
    };
    let filename = to
        .file_name()
        .ok_or_else(|| MdrefError::Path("No file name".to_string()))?;

    Ok(diff_paths(to_parent_resolved.join(filename), from_resolved).unwrap_or_default())
}

fn plan_case_only_external_replacements(
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

/// Collect all link replacements needed for internal links within the moved file itself.
fn plan_internal_replacements(
    scan_path: &Path,
    source: &Path,
    resolved_dest: &Path,
) -> Result<Vec<LinkReplacement>> {
    let links = find_links(scan_path)?;
    let mut replacements = Vec::new();
    let mut line_cache = LineCache::new();

    for link in &links {
        if let Some(replacement) =
            build_link_replacement(link, source, resolved_dest, &mut line_cache)?
        {
            replacements.push(replacement);
        }
    }

    Ok(replacements)
}

fn build_directory_path_mappings(
    source_dir: &Path,
    source_canonical: &Path,
    dest_canonical: &Path,
) -> Result<HashMap<PathBuf, PathBuf>> {
    let mut mappings = HashMap::new();
    mappings.insert(source_canonical.to_path_buf(), dest_canonical.to_path_buf());

    for entry in WalkDir::new(source_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .skip(1)
    {
        let relative = entry.path().strip_prefix(source_dir).map_err(|e| {
            MdrefError::Path(format!(
                "Cannot compute relative path for '{}' under '{}': {}",
                entry.path().display(),
                source_dir.display(),
                e
            ))
        })?;
        let old_path = entry.path().canonicalize().map_err(|e| {
            MdrefError::Path(format!(
                "Cannot canonicalize directory entry '{}': {}",
                entry.path().display(),
                e
            ))
        })?;
        mappings.insert(old_path, dest_canonical.join(relative));
    }

    Ok(mappings)
}

fn collect_markdown_files(source_dir: &Path) -> Vec<PathBuf> {
    WalkDir::new(source_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect()
}

fn resolve_reference_target(base_file: &Path, link_path_only: &str) -> Option<PathBuf> {
    if link_path_only.is_empty() {
        return None;
    }

    let decoded_link = url_decode_link(link_path_only);
    let decoded_path = Path::new(&decoded_link);
    let resolved = if decoded_path.is_absolute() {
        decoded_path.to_path_buf()
    } else {
        base_file.parent()?.join(decoded_path)
    };

    resolved.canonicalize().ok()
}

fn remap_existing_path(
    path: &Path,
    source_canonical: &Path,
    path_mappings: &HashMap<PathBuf, PathBuf>,
) -> Result<PathBuf> {
    let canonical = path.canonicalize().map_err(|e| {
        MdrefError::Path(format!(
            "Cannot canonicalize path '{}': {}",
            path.display(),
            e
        ))
    })?;

    if canonical.starts_with(source_canonical) {
        path_mappings.get(&canonical).cloned().ok_or_else(|| {
            MdrefError::Path(format!(
                "Cannot map moved path '{}' to its destination",
                path.display()
            ))
        })
    } else {
        Ok(path.to_path_buf())
    }
}

fn build_replacement_for_target(
    reference: &Reference,
    file_after_move: &Path,
    new_target: &Path,
    line_cache: &mut LineCache,
) -> Result<LinkReplacement> {
    let (_link_path_only, anchor) = split_link_and_anchor(&reference.link_text);
    let new_link_path = relative_path(file_after_move, new_target)?;

    let new_link_with_anchor = match anchor {
        Some(anchor) => format!("{}#{}", new_link_path.display(), anchor),
        None => new_link_path.display().to_string(),
    };

    build_replacement(reference, &new_link_with_anchor, line_cache)
}

fn plan_directory_replacements(
    source_dir: &Path,
    source_canonical: &Path,
    dest_canonical: &Path,
    root: &Path,
) -> Result<(ReplacementPlan, SnapshotPaths)> {
    let path_mappings =
        build_directory_path_mappings(source_dir, source_canonical, dest_canonical)?;
    let mut replacements_by_file: ReplacementPlan = HashMap::new();
    let mut snapshot_paths: HashSet<PathBuf> = HashSet::new();
    let mut line_cache = LineCache::new();

    for reference in find_references(source_dir, root)? {
        let (link_path_only, _) = split_link_and_anchor(&reference.link_text);
        let Some(old_target) = resolve_reference_target(&reference.path, link_path_only) else {
            continue;
        };
        let Some(new_target) = path_mappings.get(&old_target) else {
            continue;
        };

        let file_after_move =
            remap_existing_path(&reference.path, source_canonical, &path_mappings)?;
        let replacement = build_replacement_for_target(
            &reference,
            &file_after_move,
            new_target,
            &mut line_cache,
        )?;

        replacements_by_file
            .entry(file_after_move)
            .or_default()
            .push(replacement);
        snapshot_paths.insert(reference.path);
    }

    for markdown_file in collect_markdown_files(source_dir) {
        let file_after_move =
            remap_existing_path(&markdown_file, source_canonical, &path_mappings)?;
        let links = find_links(&markdown_file)?;

        for link in links {
            let (link_path_only, _) = split_link_and_anchor(&link.link_text);
            let Some(target_path) = resolve_reference_target(&markdown_file, link_path_only) else {
                continue;
            };

            if target_path.starts_with(source_canonical) {
                continue;
            }

            let replacement = build_replacement_for_target(
                &link,
                &file_after_move,
                &target_path,
                &mut line_cache,
            )?;
            replacements_by_file
                .entry(file_after_move.clone())
                .or_default()
                .push(replacement);
            snapshot_paths.insert(markdown_file.clone());
        }
    }

    Ok((replacements_by_file, snapshot_paths.into_iter().collect()))
}

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
pub fn mv<P, B, D>(source: P, dest: B, root: D, dry_run: bool) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<Path>,
    D: AsRef<Path>,
{
    let source = source.as_ref();
    let dest = dest.as_ref();
    let root = root.as_ref();

    if source.is_dir() {
        return mv_directory(source, dest, root, dry_run);
    }

    mv_regular_file(source, dest, root, dry_run)
}

fn mv_regular_file(source: &Path, dest: &Path, root: &Path, dry_run: bool) -> Result<()> {
    if let Some(case_only_dest) = resolve_case_only_destination(source, dest)? {
        return mv_case_only_file(source, &case_only_dest, root, dry_run);
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
    let references = find_references(source, root)?;
    let mut replacements_by_file = plan_external_replacements(&references, &resolved_dest)?;

    if dry_run {
        let internal_replacements = plan_internal_replacements(source, source, &resolved_dest)?;
        if !internal_replacements.is_empty() {
            replacements_by_file
                .entry(resolved_dest.clone())
                .or_default()
                .extend(internal_replacements);
        }
        print_dry_run_report(source, &resolved_dest, &replacements_by_file);
        return Ok(());
    }

    // Phase 2: Execute — all mutations are tracked for rollback.
    let mut transaction = MoveTransaction::new(source.to_path_buf(), resolved_dest.clone());

    // Snapshot all files that will be modified before touching anything.
    for file_path in replacements_by_file.keys() {
        transaction.snapshot_file(file_path)?;
    }

    // Ensure the parent directory of the destination exists.
    if let Some(parent) = resolved_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Copy source to destination.
    fs::copy(source, &resolved_dest)?;
    transaction.mark_copied();

    // Compute internal link replacements from the newly copied file.
    let internal_replacements = plan_internal_replacements(&resolved_dest, source, &resolved_dest)?;
    if !internal_replacements.is_empty() {
        // Snapshot the destination file (the copy) before modifying it.
        transaction.snapshot_file(&resolved_dest)?;
        replacements_by_file
            .entry(resolved_dest.clone())
            .or_default()
            .extend(internal_replacements);
    }

    // Apply all replacements within a rollback-protected context.
    execute_with_rollback(&transaction, || {
        for (file_path, replacements) in &replacements_by_file {
            apply_replacements(file_path, replacements)?;
        }
        Ok(())
    })?;

    // Remove the original file.
    fs::remove_file(source)?;
    transaction.mark_source_removed();

    Ok(())
}

fn mv_case_only_file(
    source: &Path,
    resolved_dest: &Path,
    root: &Path,
    dry_run: bool,
) -> Result<()> {
    let references = find_references(source, root)?;
    let mut replacements_by_file =
        plan_case_only_external_replacements(&references, resolved_dest)?;

    if let Some(source_replacements) = replacements_by_file.remove(source) {
        replacements_by_file
            .entry(resolved_dest.to_path_buf())
            .or_default()
            .extend(source_replacements);
    }

    if dry_run {
        print_dry_run_report(source, resolved_dest, &replacements_by_file);
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

fn mv_directory(source_dir: &Path, new_path: &Path, root: &Path, dry_run: bool) -> Result<()> {
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

    let (replacements_by_file, snapshot_paths) =
        plan_directory_replacements(source_dir, &source_canonical, &dest_canonical, root)?;

    if dry_run {
        print_dry_run_report(source_dir, &resolved_dest, &replacements_by_file);
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

/// Print a human-readable report of all changes that would be made during a move operation.
fn print_dry_run_report(
    source: &Path,
    destination: &Path,
    replacements_by_file: &HashMap<PathBuf, Vec<LinkReplacement>>,
) {
    println!(
        "[dry-run] Would move: {} -> {}",
        source.display(),
        destination.display()
    );

    if replacements_by_file.is_empty() {
        println!("[dry-run] No references to update.");
        return;
    }

    for (file_path, replacements) in replacements_by_file {
        let label = if file_path == destination {
            "Would update links in moved file"
        } else {
            "Would update reference in"
        };
        println!("[dry-run] {} {}:", label, file_path.display());
        for replacement in replacements {
            println!(
                "  Line {}: {} -> {}",
                replacement.line, replacement.old_pattern, replacement.new_pattern
            );
        }
    }
}

/// Split a link into the path part and the anchor (fragment) part.
/// Returns (path, Some(anchor)) if there's an anchor, or (path, None) if not.
/// Examples:
///   "file.md#section" -> ("file.md", Some("section"))
///   "file.md" -> ("file.md", None)
///   "#section" -> ("", Some("section"))  (pure anchor link)
fn split_link_and_anchor(link: &str) -> (&str, Option<&str>) {
    match link.find('#') {
        Some(pos) => {
            let (path, anchor) = link.split_at(pos);
            // Remove the '#' prefix from anchor
            (path, Some(&anchor[1..]))
        }
        None => (link, None),
    }
}

/// Build a LinkReplacement for an internal link in the moved file.
/// Returns `None` if the link is an external URL or a broken link that cannot be resolved.
fn build_link_replacement(
    r: &Reference,
    raw_filepath: &Path,
    new_filepath: &Path,
    line_cache: &mut LineCache,
) -> Result<Option<LinkReplacement>> {
    // External URLs (https://, http://, etc.) are not local file paths
    // and should not be rewritten during a file move.
    if is_external_url(&r.link_text) {
        return Ok(None);
    }

    // Strip anchor from link text so canonicalize works on the file path only.
    let (link_path_only, anchor) = split_link_and_anchor(&r.link_text);

    // Pure anchor links (e.g. "#section") are internal to the document
    // and should not be rewritten during a file move.
    if link_path_only.is_empty() {
        return Ok(None);
    }

    let parent = raw_filepath
        .parent()
        .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;

    // Resolve the link path; skip broken links that cannot be canonicalized.
    let current_link_absolute_path = match parent.join(link_path_only).canonicalize() {
        Ok(p) => p,
        Err(_) => return Ok(None),
    };
    let new_file_absolute_path = if new_filepath.exists() {
        new_filepath.canonicalize()?
    } else {
        let parent = new_filepath
            .parent()
            .ok_or_else(|| MdrefError::Path("No parent directory".to_string()))?;
        let parent_canonical = if parent.exists() {
            parent.canonicalize()?
        } else {
            parent.to_path_buf()
        };
        let filename = new_filepath
            .file_name()
            .ok_or_else(|| MdrefError::Path("No file name".to_string()))?;
        parent_canonical.join(filename)
    };
    let raw_file_canonical = raw_filepath.canonicalize()?;

    let new_link_path = if current_link_absolute_path == raw_file_canonical {
        PathBuf::from(
            new_file_absolute_path
                .file_name()
                .ok_or_else(|| MdrefError::Path("No file name".to_string()))?,
        )
    } else {
        relative_path(&new_file_absolute_path, &current_link_absolute_path)?
    };

    // Reconstruct the new pattern with anchor preserved.
    let new_link_with_anchor = match anchor {
        Some(a) => format!("{}#{}", new_link_path.display(), a),
        None => new_link_path.display().to_string(),
    };

    Ok(Some(build_replacement(
        r,
        &new_link_with_anchor,
        line_cache,
    )?))
}

fn build_replacement(
    reference: &Reference,
    new_url: &str,
    line_cache: &mut LineCache,
) -> Result<LinkReplacement> {
    match reference.link_type {
        LinkType::Inline => Ok(LinkReplacement {
            line: reference.line,
            column: reference.column,
            old_pattern: format!("]({})", reference.link_text),
            new_pattern: format!("]({})", new_url),
        }),
        LinkType::ReferenceDefinition => {
            build_reference_definition_replacement(reference, new_url, line_cache)
        }
    }
}

fn build_reference_definition_replacement(
    reference: &Reference,
    new_url: &str,
    line_cache: &mut LineCache,
) -> Result<LinkReplacement> {
    let line = get_cached_line(&reference.path, reference.line, line_cache)?;
    let (url_start, url_end) = find_reference_definition_url_span(line).ok_or_else(|| {
        MdrefError::Path(format!(
            "Could not parse reference definition in line {} of file {}",
            reference.line,
            reference.path.display()
        ))
    })?;

    Ok(LinkReplacement {
        line: reference.line,
        column: url_start + 1,
        old_pattern: line[url_start..url_end].to_string(),
        new_pattern: new_url.to_string(),
    })
}

fn get_cached_line<'a>(
    path: &Path,
    line_number: usize,
    line_cache: &'a mut LineCache,
) -> Result<&'a str> {
    let lines = match line_cache.entry(path.to_path_buf()) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => {
            let content = fs::read_to_string(path)?;
            entry.insert(content.lines().map(|line| line.to_string()).collect())
        }
    };

    if line_number == 0 || line_number > lines.len() {
        return Err(MdrefError::InvalidLine(format!(
            "Line number {} out of range for file {}",
            line_number,
            path.display()
        )));
    }

    Ok(lines[line_number - 1].as_str())
}

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

fn strip_utf8_bom_prefix(line: &str) -> (&str, usize) {
    match line.strip_prefix('\u{feff}') {
        Some(stripped) => (stripped, '\u{feff}'.len_utf8()),
        None => (line, 0),
    }
}

fn find_reference_definition_url_span(line: &str) -> Option<(usize, usize)> {
    let (line_without_bom, bom_offset) = strip_utf8_bom_prefix(line);
    let trimmed = line_without_bom.trim_start();
    let leading_spaces = line_without_bom.len() - trimmed.len();
    if leading_spaces > 3 || !trimmed.starts_with('[') {
        return None;
    }

    let label_end = trimmed.find("]:")?;
    if label_end == 0 {
        return None;
    }

    let after_colon_start = bom_offset + leading_spaces + label_end + 2;
    let after_colon = &line[after_colon_start..];
    let trimmed_after_colon = after_colon.trim_start();
    if trimmed_after_colon.is_empty() {
        return None;
    }

    let leading_after_colon = after_colon.len() - trimmed_after_colon.len();
    let url_start = after_colon_start + leading_after_colon;

    if let Some(stripped) = trimmed_after_colon.strip_prefix('<') {
        let end = stripped.find('>')?;
        let inner_start = url_start + 1;
        let inner_end = inner_start + end;
        Some((inner_start, inner_end))
    } else {
        let end = trimmed_after_colon
            .find(char::is_whitespace)
            .unwrap_or(trimmed_after_colon.len());
        Some((url_start, url_start + end))
    }
}

/// Apply all pending replacements to a single file in one read-write cycle.
/// Replacements are sorted in reverse order (by line desc, then column desc) so that
/// earlier replacements do not shift the positions of later ones.
fn apply_replacements(file_path: &Path, replacements: &[LinkReplacement]) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
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
            return Err(MdrefError::InvalidLine(format!(
                "Line number {} out of range for file {}",
                replacement.line,
                file_path.display()
            )));
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
            return Err(MdrefError::Path(format!(
                "Could not find link '{}' in line {} of file {}",
                replacement.old_pattern,
                replacement.line,
                file_path.display()
            )));
        }
    }

    let new_content = lines
        .into_iter()
        .map(|(line, ending)| format!("{line}{}", ending.as_str()))
        .collect::<String>();
    fs::write(file_path, new_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    #[allow(clippy::unwrap_used)]
    fn write_file(path: &str, content: &str) {
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

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
            Err(MdrefError::InvalidLine(message)) => {
                assert!(message.contains("999"));
                assert!(message.contains("doc.md"));
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
            Err(MdrefError::Path("simulated move failure".to_string()))
        });

        match result {
            Err(MdrefError::RollbackFailed {
                original_error,
                rollback_errors,
            }) => {
                assert_eq!(original_error, "Path error: simulated move failure");
                assert_eq!(rollback_errors.len(), 1);
                assert!(rollback_errors[0].contains("Failed to restore"));
                assert!(rollback_errors[0].contains("index.md"));
            }
            other => panic!("expected rollback failed error, got {other:?}"),
        }
    }
}
