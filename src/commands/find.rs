use mdref::{find_links, find_references};

pub fn run(file_path: String, root_dir: Option<String>) {
    let root_path = root_dir.unwrap_or_else(|| ".".to_string());

    // Find references to the specified file.
    match find_references(&file_path, &root_path) {
        Ok(references) => {
            if references.is_empty() {
                println!("No references found for {file_path}");
            } else {
                println!("References to {file_path}:");
                for reference in references {
                    println!("{}", reference);
                }
            }
        }
        Err(e) => {
            eprintln!("Error finding references for {file_path}: {}", e);
        }
    }

    // Find all links within the specified file.
    match find_links(&file_path) {
        Ok(links) => {
            if links.is_empty() {
                println!("No links found in {file_path}");
            } else {
                println!("Links in {file_path}:");
                for link in links {
                    println!("{}", link);
                }
            }
        }
        Err(e) => {
            eprintln!("Error finding links in {file_path}: {}", e);
        }
    }
}
