use mdref::mv_file;

pub fn run(source: String, dest: String, root: Option<String>) {
    let root = root.unwrap_or_else(|| ".".to_string());
    println!("Move {source} -> {dest} in {root}");
    mv_file(&source, &dest, &root);
}
