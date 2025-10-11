pub use thiserror::Error;

#[derive(Error, Debug)]
pub enum MdrefError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Invalid line number: {0}")]
    InvalidLine(String),
}

pub type Result<T> = std::result::Result<T, MdrefError>;
