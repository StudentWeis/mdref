use mdref::rename_file;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

// Test helper function: create test file
#[allow(clippy::unwrap_used)]
fn write_file<P: AsRef<Path>>(path: P, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

// ============= Basic rename tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_basic() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.md");
    write_file(&source, "# Source File\n\nSome content.");

    let result = rename_file(&source, "renamed.md", temp_dir.path(), false);

    assert!(result.is_ok());
    assert!(!source.exists());
    assert!(temp_dir.path().join("renamed.md").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_content() {
    let temp_dir = TempDir::new().unwrap();
    let content = "# Title\n\nParagraph with **bold** and *italic*.\n\n- item 1\n- item 2";
    let source = temp_dir.path().join("doc.md");
    write_file(&source, content);

    rename_file(&source, "doc_renamed.md", temp_dir.path(), false).unwrap();

    let renamed_path = temp_dir.path().join("doc_renamed.md");
    let result_content = fs::read_to_string(&renamed_path).unwrap();
    assert!(result_content.contains("# Title"));
    assert!(result_content.contains("**bold**"));
    assert!(result_content.contains("- item 1"));
}

// ============= Rename with references =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_external_references() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("original.md");
    write_file(&source, "# Original");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "See [original doc](original.md) for details.");

    rename_file(&source, "updated.md", temp_dir.path(), false).unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("updated.md"));
    assert!(!ref_content.contains("original.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_multiple_external_references() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("target.md");
    write_file(&source, "# Target");

    let ref1 = temp_dir.path().join("ref1.md");
    let ref2 = temp_dir.path().join("ref2.md");
    let ref3 = temp_dir.path().join("sub").join("ref3.md");
    write_file(&ref1, "[Link](target.md)");
    write_file(&ref2, "[Another](target.md)");
    write_file(&ref3, "[Deep](../target.md)");

    rename_file(&source, "new_target.md", temp_dir.path(), false).unwrap();

    let ref1_content = fs::read_to_string(&ref1).unwrap();
    let ref2_content = fs::read_to_string(&ref2).unwrap();
    let ref3_content = fs::read_to_string(&ref3).unwrap();

    assert!(ref1_content.contains("new_target.md"));
    assert!(ref2_content.contains("new_target.md"));
    assert!(ref3_content.contains("new_target.md"));
}

// ============= Rename with internal links =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_self_reference() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("page.md");
    write_file(&source, "[Self link](page.md)");

    rename_file(&source, "page_v2.md", temp_dir.path(), false).unwrap();

    let renamed_path = temp_dir.path().join("page_v2.md");
    let content = fs::read_to_string(&renamed_path).unwrap();
    assert!(content.contains("page_v2.md"));
    assert!(!content.contains("page.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_internal_links_to_other_files() {
    let temp_dir = TempDir::new().unwrap();

    let other = temp_dir.path().join("other.md");
    write_file(&other, "# Other");

    let source = temp_dir.path().join("source.md");
    write_file(&source, "[Other file](other.md)");

    rename_file(&source, "source_v2.md", temp_dir.path(), false).unwrap();

    let renamed_path = temp_dir.path().join("source_v2.md");
    let content = fs::read_to_string(&renamed_path).unwrap();
    // Renamed in same directory, so link to other.md should remain unchanged
    assert!(content.contains("other.md"));
}

// ============= Rename in subdirectory =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_in_subdirectory() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("sub").join("deep.md");
    write_file(&source, "# Deep file");

    let ref_file = temp_dir.path().join("root.md");
    write_file(&ref_file, "[Deep](sub/deep.md)");

    rename_file(&source, "shallow.md", temp_dir.path(), false).unwrap();

    assert!(temp_dir.path().join("sub").join("shallow.md").exists());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("sub/shallow.md"));
    assert!(!ref_content.contains("sub/deep.md"));
}

// ============= Error cases =============

#[test]
fn test_rename_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    let result = rename_file(
        temp_dir.path().join("ghost.md"),
        "new.md",
        temp_dir.path(),
        false,
    );
    assert!(result.is_err());
}
