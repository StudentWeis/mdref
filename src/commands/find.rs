use mdref::find_references;
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
                    println!("  {}:{}:{} - {}", reference.path.display(), reference.line, reference.column, reference.link_text);
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
}
