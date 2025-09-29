use mdref::{find_links, find_references};
use std::path::PathBuf;

pub fn run(filepath: String, root: Option<String>) {
    let root_path = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let file_path = PathBuf::from(&filepath);

    match find_references(&file_path, &root_path) {
        Ok(references) => {
            if references.is_empty() {
                println!("No references found for {}", file_path.display());
            } else {
                println!("References to {}:", file_path.display());
                for reference in references {
                    println!("{}", reference);
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Error finding references for {}: {}",
                file_path.display(),
                e
            );
        }
    }

    match find_links(&file_path) {
        Ok(links) => {
            if links.is_empty() {
                println!("No links found in {}", file_path.display());
            } else {
                println!("Links in {}:", file_path.display());
                for link in links {
                    println!("{}", link);
                }
            }
        }
        Err(e) => {
            eprintln!("Error finding links in {}: {}", file_path.display(), e);
        }
    }
}
