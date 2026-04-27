//! Replacement planning: turn a source→destination move into a concrete set of
//! per-file link rewrites, without touching the filesystem.
//!
//! This is the "pure computation" phase. Its inputs are the list of `Reference`s
//! discovered by `find_references` plus the source/destination paths; its output
//! is a [`ReplacementPlan`] that the apply phase executes.
//!
//! The module is internally organized into three sub-groups:
//!
//! - top-level planners: `plan_external_replacements`, `plan_internal_replacements`,
//!   `plan_directory_replacements`
//! - per-reference construction: `build_link_replacement`, `build_replacement`,
//!   `build_reference_definition_replacement`, `split_link_and_anchor`
//! - `ReplacementPlan` bookkeeping: `extend_unique_replacements`,
//!   `move_source_replacements_to_destination`, `add_destination_replacements`

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{
    LinkType, MdrefError, Reference, Result,
    core::{
        find::find_references,
        model::LinkReplacement,
        progress::ProgressReporter,
        util::{
            collect_markdown_files, is_external_url, relative_path, strip_utf8_bom_prefix,
            url_decode_link,
        },
    },
    find_links,
};

pub(super) type ReplacementPlan = HashMap<PathBuf, Vec<LinkReplacement>>;
pub(super) type SnapshotPaths = Vec<PathBuf>;
pub(super) type LineCache = HashMap<PathBuf, Vec<String>>;

// ============= Top-level planners =============

/// Collect all link replacements needed for external references (other files pointing to the moved file).
pub(super) fn plan_external_replacements(
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

/// Collect all link replacements needed for internal links within the moved file itself.
pub(super) fn plan_internal_replacements(
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

/// Plan replacements for a whole-directory move.
///
/// Returns `(plan, snapshot_paths)`: the plan is keyed by the files'
/// **post-move** paths, while `snapshot_paths` lists the **pre-move** paths
/// that need to be snapshotted for rollback.
pub(super) fn plan_directory_replacements(
    source_dir: &Path,
    source_canonical: &Path,
    dest_canonical: &Path,
    root: &Path,
    progress: &dyn ProgressReporter,
) -> Result<(ReplacementPlan, SnapshotPaths)> {
    let path_mappings =
        build_directory_path_mappings(source_dir, source_canonical, dest_canonical)?;
    let mut replacements_by_file: ReplacementPlan = HashMap::new();
    let mut snapshot_paths: HashSet<PathBuf> = HashSet::new();
    let mut line_cache = LineCache::new();

    progress.set_message("Scanning references...");
    for reference in find_references(source_dir, root, progress)? {
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

// ============= Directory-move internals =============

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
        let relative =
            entry
                .path()
                .strip_prefix(source_dir)
                .map_err(|e| MdrefError::PathValidation {
                    path: entry.path().to_path_buf(),
                    details: format!(
                        "cannot compute relative path under '{}': {e}",
                        source_dir.display()
                    ),
                })?;
        let old_path = entry
            .path()
            .canonicalize()
            .map_err(|e| MdrefError::PathValidation {
                path: entry.path().to_path_buf(),
                details: format!("cannot canonicalize directory entry: {e}"),
            })?;
        mappings.insert(old_path, dest_canonical.join(relative));
    }

    Ok(mappings)
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
    let canonical = path
        .canonicalize()
        .map_err(|e| MdrefError::PathValidation {
            path: path.to_path_buf(),
            details: format!("cannot canonicalize path: {e}"),
        })?;

    if canonical.starts_with(source_canonical) {
        path_mappings
            .get(&canonical)
            .cloned()
            .ok_or_else(|| MdrefError::PathValidation {
                path: path.to_path_buf(),
                details: "cannot map moved path to its destination".to_string(),
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

// ============= Per-reference replacement construction =============

/// Split a link into the path part and the anchor (fragment) part.
/// Returns (path, Some(anchor)) if there's an anchor, or (path, None) if not.
/// Examples:
///   "file.md#section" -> ("file.md", Some("section"))
///   "file.md" -> ("file.md", None)
///   "#section" -> ("", Some("section"))  (pure anchor link)
pub(super) fn split_link_and_anchor(link: &str) -> (&str, Option<&str>) {
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
pub(super) fn build_link_replacement(
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
        .ok_or_else(|| MdrefError::PathValidation {
            path: raw_filepath.to_path_buf(),
            details: "no parent directory".to_string(),
        })?;

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
            .ok_or_else(|| MdrefError::PathValidation {
                path: new_filepath.to_path_buf(),
                details: "no parent directory".to_string(),
            })?;
        let parent_canonical = if parent.exists() {
            parent.canonicalize()?
        } else {
            parent.to_path_buf()
        };
        let filename = new_filepath
            .file_name()
            .ok_or_else(|| MdrefError::PathValidation {
                path: new_filepath.to_path_buf(),
                details: "no file name".to_string(),
            })?;
        parent_canonical.join(filename)
    };
    let raw_file_canonical = raw_filepath.canonicalize()?;

    let new_link_path = if current_link_absolute_path == raw_file_canonical {
        PathBuf::from(new_file_absolute_path.file_name().ok_or_else(|| {
            MdrefError::PathValidation {
                path: new_file_absolute_path.clone(),
                details: "no file name".to_string(),
            }
        })?)
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

pub(super) fn build_replacement(
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
    let (url_start, url_end) =
        find_reference_definition_url_span(line).ok_or_else(|| MdrefError::PathValidation {
            path: reference.path.clone(),
            details: format!(
                "could not parse reference definition in line {}",
                reference.line
            ),
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
            let content = fs::read_to_string(path).map_err(|e| MdrefError::IoRead {
                path: path.to_path_buf(),
                source: e,
            })?;
            entry.insert(content.lines().map(|line| line.to_string()).collect())
        }
    };

    if line_number == 0 || line_number > lines.len() {
        return Err(MdrefError::InvalidLineReference {
            path: path.to_path_buf(),
            line: line_number,
            details: format!("line number out of range (file has {} lines)", lines.len()),
        });
    }

    Ok(lines[line_number - 1].as_str())
}

/// Locate the URL span inside a Markdown reference definition line.
///
/// Returns `(url_start, url_end)` as byte offsets into `line`, or `None` if the
/// line is not a valid reference definition (more than 3 leading spaces, no
/// `[label]:` prefix, empty URL, etc.). Angle-bracket-wrapped URLs have the
/// brackets excluded from the span.
pub(super) fn find_reference_definition_url_span(line: &str) -> Option<(usize, usize)> {
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

// ============= ReplacementPlan bookkeeping =============

fn extend_unique_replacements(
    destination_replacements: &mut Vec<LinkReplacement>,
    replacements: Vec<LinkReplacement>,
) {
    for replacement in replacements {
        let already_present = destination_replacements.iter().any(|existing| {
            existing.line == replacement.line
                && existing.column == replacement.column
                && existing.old_pattern == replacement.old_pattern
                && existing.new_pattern == replacement.new_pattern
        });

        if !already_present {
            destination_replacements.push(replacement);
        }
    }
}

/// Move all replacements keyed on `source` onto the `destination` bucket.
///
/// Used during case-only renames, where references originally keyed by the
/// pre-rename path must follow the file to its new (post-rename) path so the
/// apply phase only writes to the moved file once.
pub(super) fn move_source_replacements_to_destination(
    replacements_by_file: &mut ReplacementPlan,
    source: &Path,
    destination: &Path,
) {
    if let Some(source_replacements) = replacements_by_file.remove(source) {
        let destination_replacements = replacements_by_file
            .entry(destination.to_path_buf())
            .or_default();
        extend_unique_replacements(destination_replacements, source_replacements);
    }
}

/// Append replacements for the moved file itself (its internal links) onto the
/// destination bucket, deduplicating against anything already planned there.
pub(super) fn add_destination_replacements(
    replacements_by_file: &mut ReplacementPlan,
    destination: &Path,
    replacements: Vec<LinkReplacement>,
) {
    if replacements.is_empty() {
        return;
    }

    let destination_replacements = replacements_by_file
        .entry(destination.to_path_buf())
        .or_default();
    extend_unique_replacements(destination_replacements, replacements);
}
