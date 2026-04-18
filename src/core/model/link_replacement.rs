/// A pending replacement: which line/column to find the old pattern, and what to replace it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkReplacement {
    pub line: usize,
    pub column: usize,
    pub old_pattern: String,
    pub new_pattern: String,
}
