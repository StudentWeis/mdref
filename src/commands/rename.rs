use std::path::PathBuf;

pub fn run(old: String, new: String, root: Option<String>) {
    let root_path = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Placeholder: implement rename logic here.
    // For now we just print out the intended operation.
    println!("(rename) Rename {} -> {} in {}", old, new, root_path.display());

    // TODO: Update markdown links similar to `find.rs` logic.
}
