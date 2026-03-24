use std::{fs, io::Write, path::Path};

#[allow(clippy::unwrap_used)]
pub fn write_file<P: AsRef<Path>>(path: P, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}
