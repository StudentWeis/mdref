use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

/// Struct to hold reference information
#[derive(Debug)]
pub struct Reference {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub link_text: String,
}

impl Reference {
    /// Constructor for References
    pub fn new(path: PathBuf, line: usize, column: usize, link_text: String) -> Self {
        Self {
            path,
            line,
            column,
            link_text,
        }
    }
}

impl Display for Reference {
    /// Format as "path:line:column - link_text"
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{}:{} - {}",
            self.path.display(),
            self.line,
            self.column,
            self.link_text
        )
    }
}
