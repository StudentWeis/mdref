use mdref::{Result, find_links, find_references};

pub fn run(path: String, root_dir: Option<String>) -> Result<()> {
    let root_path = root_dir.unwrap_or_else(|| ".".to_string());

    println!("-------------------------------");

    // Find references to the specified file.
    let references = find_references(&path, &root_path)?;
    if references.is_empty() {
        println!("No references found for {path}");
    } else {
        println!("References to {path}:");
        for reference in references {
            println!("{}", reference);
        }
    }

    println!("-------------------------------");

    // Find all links within the specified file.
    let links = find_links(&path)?;
    if links.is_empty() {
        println!("No links found in {path}");
    } else {
        println!("Links in {path}:");
        for link in links {
            println!("{}", link);
        }
    }

    println!("-------------------------------");

    Ok(())
}
