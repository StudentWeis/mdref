use std::{io::Write, path::Path};

use clap::{Subcommand, ValueEnum};
use mdref::{
    MdrefError, Result,
    core::model::{LinkReplacement, MoveChange, MoveChangeKind, MovePreview},
};
use serde::Serialize;

mod find;
mod mv;
pub(crate) mod progress;
mod rename;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Find references to a file
    Find {
        /// The path to find references
        path: String,
        /// Root directory to search in (default: current directory)
        #[arg(short, long)]
        root: Option<String>,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Rename a file and update references
    Rename {
        /// Old filename
        old: String,
        /// New filename
        new: String,
        /// Root directory
        #[arg(short, long)]
        root: Option<String>,
        /// Preview changes without modifying any files
        #[arg(long)]
        dry_run: bool,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
    /// Move a file and update references
    Mv {
        /// Source path
        source: String,
        /// Destination path
        dest: String,
        /// Root directory
        #[arg(short, long)]
        root: Option<String>,
        /// Preview changes without modifying any files
        #[arg(long)]
        dry_run: bool,
        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
        format: OutputFormat,
    },
}

pub struct CommandOutputContext {
    pub operation: &'static str,
    pub format: OutputFormat,
    pub target: Option<String>,
    pub source: Option<String>,
    pub destination: Option<String>,
    pub new_name: Option<String>,
    pub root: Option<String>,
    pub dry_run: Option<bool>,
}

#[derive(Serialize)]
pub struct JsonReplacement {
    pub line: usize,
    pub column: usize,
    pub old_pattern: String,
    pub new_pattern: String,
}

#[derive(Serialize)]
pub struct JsonMoveChange {
    pub path: String,
    pub kind: &'static str,
    pub replacements: Vec<JsonReplacement>,
}

impl Commands {
    pub fn output_context(&self) -> CommandOutputContext {
        match self {
            Self::Find { path, root, format } => CommandOutputContext {
                operation: "find",
                format: *format,
                target: Some(path.clone()),
                source: None,
                destination: None,
                new_name: None,
                root: root.clone(),
                dry_run: None,
            },
            Self::Rename {
                old,
                new,
                root,
                dry_run,
                format,
            } => CommandOutputContext {
                operation: "rename",
                format: *format,
                target: None,
                source: Some(old.clone()),
                destination: Some(Path::new(old).with_file_name(new).display().to_string()),
                new_name: Some(new.clone()),
                root: root.clone(),
                dry_run: Some(*dry_run),
            },
            Self::Mv {
                source,
                dest,
                root,
                dry_run,
                format,
            } => CommandOutputContext {
                operation: "mv",
                format: *format,
                target: None,
                source: Some(source.clone()),
                destination: Some(dest.clone()),
                new_name: None,
                root: root.clone(),
                dry_run: Some(*dry_run),
            },
        }
    }
}

pub fn handle_command(command: Commands, progress: bool) -> Result<()> {
    match command {
        Commands::Find {
            path: filepath,
            root,
            format,
        } => find::run(filepath, root, progress, format),
        Commands::Rename {
            old,
            new,
            root,
            dry_run,
            format,
        } => rename::run(old, new, root, dry_run, progress, format),
        Commands::Mv {
            source,
            dest,
            root,
            dry_run,
            format,
        } => mv::run(source, dest, root, dry_run, progress, format),
    }
}

pub fn write_json_output<W: Write, T: Serialize>(writer: &mut W, payload: &T) -> Result<()> {
    serde_json::to_writer_pretty(&mut *writer, payload).map_err(|error| {
        MdrefError::SerializationFailed {
            details: format!("failed to write JSON output: {error}"),
        }
    })?;
    writeln!(writer)?;
    Ok(())
}

pub fn write_move_preview_human<W: Write>(preview: &MovePreview, writer: &mut W) -> Result<()> {
    writeln!(
        writer,
        "[dry-run] Would move: {} -> {}",
        preview.source.display(),
        preview.destination.display()
    )?;

    if preview.changes.is_empty() {
        writeln!(writer, "[dry-run] No references to update.")?;
        return Ok(());
    }

    for change in &preview.changes {
        let label = match change.kind {
            MoveChangeKind::MovedFileUpdate => "Would update links in moved file",
            MoveChangeKind::ReferenceUpdate => "Would update reference in",
        };
        writeln!(writer, "[dry-run] {} {}:", label, change.path.display())?;
        for replacement in &change.replacements {
            writeln!(
                writer,
                "  Line {}: {} -> {}",
                replacement.line, replacement.old_pattern, replacement.new_pattern
            )?;
        }
    }

    Ok(())
}

pub fn json_move_changes(preview: &MovePreview) -> Vec<JsonMoveChange> {
    preview.changes.iter().map(JsonMoveChange::from).collect()
}

impl From<&MoveChange> for JsonMoveChange {
    fn from(change: &MoveChange) -> Self {
        Self {
            path: change.path.display().to_string(),
            kind: match change.kind {
                MoveChangeKind::ReferenceUpdate => "reference_update",
                MoveChangeKind::MovedFileUpdate => "moved_file_update",
            },
            replacements: change
                .replacements
                .iter()
                .map(JsonReplacement::from)
                .collect(),
        }
    }
}

impl From<&LinkReplacement> for JsonReplacement {
    fn from(replacement: &LinkReplacement) -> Self {
        Self {
            line: replacement.line,
            column: replacement.column,
            old_pattern: replacement.old_pattern.clone(),
            new_pattern: replacement.new_pattern.clone(),
        }
    }
}
