use std::path::PathBuf;

pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum MdrefError {
    #[error("IO error reading '{path}': {source}")]
    IoRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("IO error writing '{path}': {source}")]
    IoWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error for '{path}': {details}")]
    PathValidation { path: PathBuf, details: String },

    #[error("Invalid line reference at {path}:{line}: {details}")]
    InvalidLineReference {
        path: PathBuf,
        line: usize,
        details: String,
    },

    #[error("Serialization failed: {details}")]
    SerializationFailed { details: String },

    #[error("Operation failed and rollback also failed: {original_error}; rollback errors: {}", rollback_errors.join("; "))]
    RollbackFailed {
        original_error: String,
        rollback_errors: Vec<String>,
    },
}

pub type Result<T> = std::result::Result<T, MdrefError>;
