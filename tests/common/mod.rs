#![allow(clippy::unwrap_used)]
#![allow(dead_code)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mdref"))
}

pub fn run_cli(args: &[&str]) -> Output {
    Command::new(binary_path()).args(args).output().unwrap()
}

pub fn temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

pub fn read_file<P: AsRef<Path>>(path: P) -> String {
    fs::read_to_string(path).unwrap()
}

pub fn assert_file_contains<P: AsRef<Path>>(path: P, expected: &str) {
    let content = read_file(path);
    assert!(
        content.contains(expected),
        "expected file to contain `{expected}`, got: {content}"
    );
}

pub fn assert_file_not_contains<P: AsRef<Path>>(path: P, unexpected: &str) {
    let content = read_file(path);
    assert!(
        !content.contains(unexpected),
        "expected file not to contain `{unexpected}`, got: {content}"
    );
}

pub fn write_file<P: AsRef<Path>>(path: P, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}
