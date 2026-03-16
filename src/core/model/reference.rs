use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

/// The type of Markdown link that produced this reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkType {
    /// An inline link: `[text](url)` or `![alt](url)`
    Inline,
    /// A link reference definition: `[label]: url`
    ReferenceDefinition,
}

/// Struct to hold reference information
#[derive(Debug)]
pub struct Reference {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub link_text: String,
    pub link_type: LinkType,
}

impl Reference {
    /// Constructor for References (defaults to Inline link type for backward compatibility)
    pub fn new(path: PathBuf, line: usize, column: usize, link_text: String) -> Self {
        Self {
            path,
            line,
            column,
            link_text,
            link_type: LinkType::Inline,
        }
    }

    /// Constructor for References with explicit link type
    pub fn with_link_type(
        path: PathBuf,
        line: usize,
        column: usize,
        link_text: String,
        link_type: LinkType,
    ) -> Self {
        Self {
            path,
            line,
            column,
            link_text,
            link_type,
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
