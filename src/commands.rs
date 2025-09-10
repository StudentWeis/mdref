use clap::Subcommand;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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
    },
}

pub fn handle_command(command: Commands) {
    match command {
        Commands::Find { filepath, root } => {
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let file_path = if Path::new(&filepath).is_absolute() {
                PathBuf::from(&filepath)
            } else {
                root_path.join(&filepath)
            };
            find_references(&file_path, &root_path);
        }
        Commands::Rename { old, new, root } => {
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            rename_file(&old, &new, &root_path);
        }
        Commands::Mv { source, dest, root } => {
            let root_path = root
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            move_file(&source, &dest, &root_path);
        }
    }
}

fn find_references(filepath: &Path, root: &Path) {
    let target_canonical = match filepath.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            println!("File {} not found or inaccessible", filepath.display());
            return;
        }
    };

    let mut references = Vec::new();
    let link_regex = Regex::new(r"\[([^\]]*)\]\(([^)]+)\)").unwrap();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
    {
        if let Ok(content) = fs::read_to_string(entry.path()) {
            process_md_file(
                &content,
                entry.path(),
                &link_regex,
                root,
                &target_canonical,
                &mut references,
            );
        }
    }

    if references.is_empty() {
        println!("No references found for {}", filepath.display());
    } else {
        println!("References to {}:", filepath.display());
        for (file, line, link) in references {
            println!("  {}:{} - {}", file.display(), line, link);
        }
    }
}

fn process_md_file(
    content: &str,
    file_path: &Path,
    link_regex: &Regex,
    root: &Path,
    target_canonical: &Path,
    references: &mut Vec<(PathBuf, usize, String)>,
) {
    for (line_num, line) in content.lines().enumerate() {
        for cap in link_regex.captures_iter(line) {
            let link = &cap[2];
            if let Some(resolved_path) = resolve_link(file_path, link, root) {
                match resolved_path.canonicalize() {
                    Ok(canonical) if canonical == *target_canonical => {
                        references.push((file_path.to_path_buf(), line_num + 1, link.to_string()));
                    }
                    _ => {}
                }
            }
        }
    }
}

fn resolve_link(base_path: &Path, link: &str, root: &Path) -> Option<PathBuf> {
    let link_path = Path::new(link);
    if link_path.is_absolute() {
        Some(link_path.to_path_buf())
    } else {
        // Try relative to the file's directory first
        if let Some(parent) = base_path.parent() {
            let resolved = parent.join(link_path);
            if resolved.exists() {
                return Some(resolved);
            }
        }
        // If not found, try relative to the root directory
        Some(root.join(link_path))
    }
}

fn rename_file(old: &str, new: &str, root: &Path) {
    // TODO: Implement rename
    println!("Rename {} to {} in {}", old, new, root.display());
}

fn move_file(source: &str, dest: &str, root: &Path) {
    // TODO: Implement move
    println!("Move {} to {} in {}", source, dest, root.display());
}

#[test]
fn test() {
    let test = PathBuf::from("examples/main.md");
    println!("{}", test.canonicalize().unwrap().to_str().unwrap());
}
