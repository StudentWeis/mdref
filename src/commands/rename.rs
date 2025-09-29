use std::path::PathBuf;

pub fn run(old: String, new: String, root: Option<String>) {
    let root_path = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let old_path = PathBuf::from(&old);
    let new_path = old_path.with_file_name(&new);
    println!(
        "Rename {} -> {} in {}",
        old_path.display(),
        new_path.display(),
        root_path.display()
    );
    mdref::mv_file(&old_path, &new_path, &root_path);
}
