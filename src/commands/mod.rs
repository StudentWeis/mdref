use clap::{Subcommand, ValueEnum};
use mdref::Result;

mod find;
mod mv;
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
    },
}

impl Commands {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Find { .. } => "find",
            Self::Rename { .. } => "rename",
            Self::Mv { .. } => "mv",
        }
    }

    pub fn output_format(&self) -> OutputFormat {
        match self {
            Self::Find { format, .. } => *format,
            Self::Rename { .. } | Self::Mv { .. } => OutputFormat::Human,
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            Self::Find { path, .. } => Some(path.as_str()),
            Self::Rename { .. } | Self::Mv { .. } => None,
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
        } => rename::run(old, new, root, dry_run, progress),
        Commands::Mv {
            source,
            dest,
            root,
            dry_run,
        } => mv::run(source, dest, root, dry_run, progress),
    }
}
