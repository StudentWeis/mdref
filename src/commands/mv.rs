use std::path::PathBuf;

use mdref::mv_file;

pub fn run(source: String, dest: String, root: Option<String>) {
    let root_path = root
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let source_path = PathBuf::from(&source);
    let dest_path = PathBuf::from(&dest);

    println!("Move {} -> {} in {}", source, dest, root_path.display());
    mv_file(&source_path, &dest_path, &root_path);
}
