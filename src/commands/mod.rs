use clap::Subcommand;
use mdref::Result;

mod find;
mod mv;
mod rename;

#[derive(Subcommand)]
pub enum Commands {
    /// Find references to a file
    Find {
        /// The path to find references
        path: String,
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

pub fn handle_command(command: Commands, progress: bool) -> Result<()> {
    match command {
        Commands::Find {
            path: filepath,
            root,
        } => find::run(filepath, root, progress),
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
