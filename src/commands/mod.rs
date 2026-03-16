use clap::Subcommand;
use mdref::Result;

mod find;
mod mv;
mod rename;

#[derive(Subcommand)]
pub enum Commands {
    /// Find references to a file
    Find {
        /// The file path to find references for
        filepath: String,
        /// Root directory to search in (default: current directory)
        #[arg(short, long)]
        root: Option<String>,
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

pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Find { filepath, root } => find::run(filepath, root),
        Commands::Rename { old, new, root, dry_run } => rename::run(old, new, root, dry_run),
        Commands::Mv { source, dest, root, dry_run } => mv::run(source, dest, root, dry_run),
    }
}
