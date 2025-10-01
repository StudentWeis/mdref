use std::path::PathBuf;

pub fn run(old: String, new: String, root: Option<String>) {
    let root_path = root.unwrap_or_else(|| ".".to_string());
    let old_path = PathBuf::from(&old);
    let new_path = old_path.with_file_name(&new);
    println!(
        "Rename {} -> {} in {root_path}",
        old_path.display(),
        new_path.display()
    );
    mdref::mv_file(&old_path, &new_path, &root_path);
}
