use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
}

impl MoveTransaction {
    pub fn new(source_path: PathBuf, destination_path: PathBuf) -> Self {
        Self {
            file_snapshots: HashMap::new(),
            copied_destination: None,
            source_removed: false,
            source_path,
            destination_path,
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
    }

    /// Record that the source file has been removed.
    pub fn mark_source_removed(&mut self) {
        self.source_removed = true;
    }

    /// Undo all recorded mutations, returning any errors encountered during rollback.
    pub fn rollback(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // 1. Restore all modified files to their original content.
        for (path, original_content) in &self.file_snapshots {
            if let Err(err) = fs::write(path, original_content) {
                errors.push(format!("Failed to restore {}: {}", path.display(), err));
            }
        }

        // 2. If the source was deleted, restore it from the destination copy.
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

        // 3. If the destination exists and source was not deleted, remove destination.
        if !self.source_removed
            && let Some(dest) = &self.copied_destination
            && dest.exists()
            && let Err(err) = fs::remove_file(dest)
        {
            errors.push(format!(
                "Failed to remove destination {}: {}",
                dest.display(),
                err
            ));
        }

        errors
    }
}
