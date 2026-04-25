# mdref design document

mdref is a command-line tool and library for discovering, moving, and renaming Markdown references.

This document describes the system as it exists today, the boundaries that are intentionally in scope, and future candidates that are not implemented yet. Read it together with [README.md](../README.md) for user-facing usage and [DirectoryMove.md](./DirectoryMove.md) for the directory-move algorithm.

## Current implementation

### Primary workflows

- `find`: find inbound references to a Markdown file and list outbound links inside that file.
- `mv`: move a Markdown file or directory and rewrite affected local Markdown links.
- `rename`: rename a file in place by delegating to `mv` with a new filename in the same directory.

### Layering

- CLI entrypoints in `src/main.rs` and `src/commands/*` own argument parsing, progress display, and human or JSON rendering.
- The public library surface in `src/lib.rs` exposes `find_references`, `find_references_with_progress`, `mv`, `mv_with_progress`, `rename`, and `rename_with_progress`.
- Core behavior lives under `src/core`:
	- `find.rs` parses Markdown and locates references.
	- `mv.rs` validates paths, plans rewrites, executes moves, and coordinates rollback.
	- `rename.rs` is a semantic wrapper around `mv`.
	- `model/*` contains shared data structures such as move previews, replacements, and transactions.

### Reference discovery model

- Discovery is limited to Markdown files with the `.md` extension.
- Directory scans use standard ignore handling through `.gitignore` and related ignore files, and this still applies when the root is not itself a Git repository.
- `find` returns two views of the same target:
	- inbound references from other Markdown files under the chosen root
	- outbound links found inside the target file
- Supported local reference forms include inline links and link reference definitions.
- External URLs such as `https://`, `mailto:`, and similar schemes are treated as non-local and are never rewritten.
- Pure fragment links such as `#section` are not rewritten. File links with fragments keep the fragment.

### Move and rename model

- A file move updates:
	- other Markdown files that point to the moved file
	- links inside the moved file whose relative target changes after the move
- A directory move is planned before any mutation:
	- external files pointing into the moved directory are rewritten
	- moved Markdown files pointing outside the directory are rewritten
	- links between files that move together are usually left unchanged because their relative positions do not change
- `rename` is implemented as a same-directory move and therefore shares validation, rewrite planning, dry-run behavior, and rollback semantics with `mv`.
- `--dry-run` computes the full move preview without modifying files.
- Execution uses a transaction-like flow: plan first, then mutate, then attempt rollback if a later step fails.

### Output contracts

- Human output is intended for interactive use:
	- `find` prints separate sections for references and links.
	- `mv` and `rename` print a summary for real runs.
	- dry-run mode prints a preview of the move and each planned replacement.
- JSON output is available for `find`, `mv`, and `rename` and is intended for automation.
- Successful `find` output includes `operation`, `target`, `references`, and `links`.
- Successful `mv` output includes `operation`, `source`, `destination`, `root`, `dry_run`, and `changes`.
- Successful `rename` output includes `operation`, `source`, `new_name`, `destination`, `root`, `dry_run`, and `changes`.
- Each change entry includes the affected `path`, a `kind` (`reference_update` or `moved_file_update`), and line or column-based replacements.
- When JSON output is requested, command failures are also emitted as JSON on stderr with command context and an `error` message.

## Known boundaries

- The project is focused on local Markdown references. It does not try to validate or rewrite arbitrary text formats or non-Markdown documents.
- Only `.md` files discovered by the scan participate in reference discovery and rewrite planning.
- Ignored files and directories are intentionally skipped during scanning, so references inside ignored Markdown files are not updated.
- Path resolution prefers canonicalized real paths when possible. This helps with symlink-aware comparisons and paths that do not exist yet, but the exact filesystem behavior still depends on the host platform.
- Rollback is best-effort rather than a hard atomicity guarantee. The code attempts to restore moved paths and rewritten file contents, but filesystem boundaries, permissions, and platform-specific rename semantics can still limit recovery.
- Directory move behavior is described in more detail in [DirectoryMove.md](./DirectoryMove.md). This document stays at the architectural level.

## Future candidates

The following items are not part of the current implementation and should be treated as roadmap ideas, not existing capabilities:

- `validate`: verify that local Markdown references resolve successfully.
- `report`: generate a broader project-level summary of Markdown files and relationships.
- Additional reporting or machine-readable formats beyond today's human and JSON outputs.
- Broader documentation around path resolution, symlink handling, and rollback guarantees as dedicated reference material.
