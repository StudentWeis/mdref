use std::path::PathBuf;

pub fn run(source: String, dest: String, root: Option<String>) {
    let root_path = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Placeholder: implement move logic here.
    println!("(mv) Move {} -> {} in {}", source, dest, root_path.display());

    // TODO: Update markdown links similar to `find.rs` logic.
}
