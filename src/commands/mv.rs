use mdref::{Result, mv_file};

pub fn run(source: String, dest: String, root: Option<String>, dry_run: bool) -> Result<()> {
    let root = root.unwrap_or_else(|| ".".to_string());
    if !dry_run {
        println!("Move {source} -> {dest} in {root}");
    }
    mv_file(&source, &dest, &root, dry_run)
}
