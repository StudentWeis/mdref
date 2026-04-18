use std::path::PathBuf;

use super::LinkReplacement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveChangeKind {
    ReferenceUpdate,
    MovedFileUpdate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveChange {
    pub path: PathBuf,
    pub kind: MoveChangeKind,
    pub replacements: Vec<LinkReplacement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovePreview {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub changes: Vec<MoveChange>,
}
