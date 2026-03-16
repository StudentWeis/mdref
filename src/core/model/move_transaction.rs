use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum MoveKind {
    CopiedPath,
    RenamedPath,
}

/// Tracks all filesystem mutations so they can be rolled back on failure.
///
/// The transaction records three kinds of operations:
/// 1. **File snapshots** – original content of files that will be modified in-place.
/// 2. **Copied destination** – the new file created by `fs::copy`.
/// 3. **Removed source** – set after the original file is deleted, so rollback can restore it.
pub struct MoveTransaction {
    pub file_snapshots: HashMap<PathBuf, String>,
    pub copied_destination: Option<PathBuf>,
    pub source_removed: bool,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    move_kind: Option<MoveKind>,
}

impl MoveTransaction {
    pub fn new(source_path: PathBuf, destination_path: PathBuf) -> Self {
        Self {
            file_snapshots: HashMap::new(),
            copied_destination: None,
            source_removed: false,
            source_path,
            destination_path,
            move_kind: None,
        }
    }

    /// Snapshot a file's current content before modifying it.
    pub fn snapshot_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        if !self.file_snapshots.contains_key(path) {
            let content = fs::read_to_string(path)?;
            self.file_snapshots.insert(path.to_path_buf(), content);
        }
        Ok(())
    }

    /// Record that the destination file was created via copy.
    pub fn mark_copied(&mut self) {
        self.copied_destination = Some(self.destination_path.clone());
        self.move_kind = Some(MoveKind::CopiedPath);
    }

    /// Record that the source path was renamed to the destination path.
    pub fn mark_renamed(&mut self) {
        self.copied_destination = Some(self.destination_path.clone());
        self.source_removed = true;
        self.move_kind = Some(MoveKind::RenamedPath);
    }

    /// Record that the source file has been removed.
    pub fn mark_source_removed(&mut self) {
        self.source_removed = true;
    }

    /// Undo all recorded mutations, returning any errors encountered during rollback.
    pub fn rollback(&self) -> Vec<String> {
        let mut errors = Vec::new();

        match self.move_kind {
            Some(MoveKind::RenamedPath) => {
                if self.source_removed
                    && let Some(dest) = &self.copied_destination
                    && dest.exists()
                    && !self.source_path.exists()
                    && let Err(err) = fs::rename(dest, &self.source_path)
                {
                    errors.push(format!(
                        "Failed to move {} back to {}: {}",
                        dest.display(),
                        self.source_path.display(),
                        err
                    ));
                }

                for (path, original_content) in &self.file_snapshots {
                    if let Err(err) = fs::write(path, original_content) {
                        errors.push(format!("Failed to restore {}: {}", path.display(), err));
                    }
                }
            }
            _ => {
                for (path, original_content) in &self.file_snapshots {
                    if let Err(err) = fs::write(path, original_content) {
                        errors.push(format!("Failed to restore {}: {}", path.display(), err));
                    }
                }

                if self.source_removed
                    && let Some(dest) = &self.copied_destination
                    && dest.exists()
                    && let Err(err) = fs::copy(dest, &self.source_path)
                {
                    errors.push(format!(
                        "Failed to restore source {} from {}: {}",
                        self.source_path.display(),
                        dest.display(),
                        err
                    ));
                }

                if !self.source_removed
                    && let Some(dest) = &self.copied_destination
                    && dest.exists()
                    && let Err(err) = remove_path(dest)
                {
                    errors.push(format!(
                        "Failed to remove destination {}: {}",
                        dest.display(),
                        err
                    ));
                }
            }
        }

        errors
    }
}

fn remove_path(path: &std::path::Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
