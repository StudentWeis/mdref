use mdref::{Result, rename};

pub fn run(old: String, new: String, root: Option<String>, dry_run: bool) -> Result<()> {
    let root_path = root.unwrap_or_else(|| ".".to_string());
    if !dry_run {
        println!("Rename {old} -> {new} in {root_path}");
    }
    rename(&old, &new, &root_path, dry_run)
}
