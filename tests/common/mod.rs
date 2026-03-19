#![allow(clippy::unwrap_used)]
#![allow(dead_code)]

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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

pub struct SingleFileReferenceFixture {
    _temp_dir: TempDir,
    pub root: PathBuf,
    pub target: PathBuf,
    pub reference: PathBuf,
}

pub struct MultiFileReferenceFixture {
    _temp_dir: TempDir,
    pub root: PathBuf,
    pub target: PathBuf,
    pub primary_reference: PathBuf,
    pub secondary_reference: PathBuf,
    pub nested_reference: PathBuf,
}

pub struct DirectoryMoveFixture {
    _temp_dir: TempDir,
    pub root: PathBuf,
    pub source_dir: PathBuf,
    pub destination_dir: PathBuf,
    pub external_reference: PathBuf,
}

pub struct UnicodePathFixture {
    _temp_dir: TempDir,
    pub root: PathBuf,
    pub source: PathBuf,
    pub reference: PathBuf,
    pub destination: PathBuf,
}

pub fn fixture_single_file_reference() -> SingleFileReferenceFixture {
    let temp_dir = temp_dir();
    let root = temp_dir.path().to_path_buf();
    let target = root.join("target.md");
    let reference = root.join("reference.md");

    write_file(&target, "# Target");
    write_file(&reference, "See [target](target.md)");

    SingleFileReferenceFixture {
        _temp_dir: temp_dir,
        root,
        target,
        reference,
    }
}

pub fn fixture_multi_file_reference() -> MultiFileReferenceFixture {
    let temp_dir = temp_dir();
    let root = temp_dir.path().to_path_buf();
    let target = root.join("target.md");
    let primary_reference = root.join("ref1.md");
    let secondary_reference = root.join("ref2.md");
    let nested_reference = root.join("sub").join("ref3.md");

    write_file(&target, "# Target");
    write_file(&primary_reference, "[Link](target.md)");
    write_file(&secondary_reference, "[Another](target.md)");
    write_file(&nested_reference, "[Deep](../target.md)");

    MultiFileReferenceFixture {
        _temp_dir: temp_dir,
        root,
        target,
        primary_reference,
        secondary_reference,
        nested_reference,
    }
}

pub fn fixture_directory_move() -> DirectoryMoveFixture {
    let temp_dir = temp_dir();
    let root = temp_dir.path().to_path_buf();

    let source_dir = root.join("docs");
    let destination_dir = root.join("archive").join("docs");
    let guide_file = source_dir.join("guide.md");
    let topic_file = source_dir.join("nested").join("topic.md");
    let outside_file = root.join("shared").join("faq.md");
    let external_reference = root.join("index.md");

    write_file(
        &guide_file,
        "[Topic](nested/topic.md)\n\n[FAQ](../shared/faq.md)",
    );
    write_file(&topic_file, "[Guide](../guide.md)");
    write_file(&outside_file, "# FAQ");
    write_file(
        &external_reference,
        "[Guide](docs/guide.md)\n\n[Topic](docs/nested/topic.md)",
    );

    DirectoryMoveFixture {
        _temp_dir: temp_dir,
        root,
        source_dir,
        destination_dir,
        external_reference,
    }
}

pub fn fixture_unicode_paths() -> UnicodePathFixture {
    let temp_dir = temp_dir();
    let root = temp_dir.path().to_path_buf();

    let source = root.join("原始文档.md");
    let reference = root.join("索引.md");
    let destination = root.join("归档").join("更新文档.md");

    write_file(&source, "# 原始文档");
    write_file(&reference, "请查看 [原始文档](原始文档.md) 获取更多信息。");

    UnicodePathFixture {
        _temp_dir: temp_dir,
        root,
        source,
        reference,
        destination,
    }
}
