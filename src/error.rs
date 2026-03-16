pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum MdrefError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Invalid line number: {0}")]
    InvalidLine(String),

    #[error("Operation failed and rollback also failed: {original_error}; rollback errors: {}", rollback_errors.join("; "))]
    RollbackFailed {
        original_error: String,
        rollback_errors: Vec<String>,
    },
}

pub type Result<T> = std::result::Result<T, MdrefError>;
