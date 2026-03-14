use mdref::mv_file;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

// Test helper function: create temporary test environment
#[allow(clippy::unwrap_used)]
fn create_test_env(test_name: &str) -> String {
    let base_dir = format!("test_rename_{}", test_name);
    if Path::new(&base_dir).exists() {
        fs::remove_dir_all(&base_dir).ok();
    }
    fs::create_dir_all(&base_dir).unwrap();
    base_dir
}

// Test helper function: cleanup test environment
fn cleanup_test_env(test_dir: &str) {
    if Path::new(test_dir).exists() {
        fs::remove_dir_all(test_dir).ok();
    }
}

// Test helper function: create test file
#[allow(clippy::unwrap_used)]
fn create_test_file(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

// Simulate rename: compute new path and call mv_file, same as commands/rename.rs
#[allow(clippy::unwrap_used)]
fn rename_file(old_path: &str, new_name: &str, root_dir: &str) -> mdref::Result<()> {
    let old = PathBuf::from(old_path);
    let new = old.with_file_name(new_name);
    mv_file(&old, &new, root_dir)
}

// ============= Basic rename tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_basic() {
    let test_dir = create_test_env("basic");
    let source = format!("{}/source.md", test_dir);
    create_test_file(&source, "# Source File\n\nSome content.");

    let result = rename_file(&source, "renamed.md", &test_dir);

    assert!(result.is_ok());
    assert!(!Path::new(&source).exists());
    assert!(Path::new(&format!("{}/renamed.md", test_dir)).exists());

    cleanup_test_env(&test_dir);
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_content() {
    let test_dir = create_test_env("preserve");
    let content = "# Title\n\nParagraph with **bold** and *italic*.\n\n- item 1\n- item 2";
    let source = format!("{}/doc.md", test_dir);
    create_test_file(&source, content);

    rename_file(&source, "doc_renamed.md", &test_dir).unwrap();

    let renamed_path = format!("{}/doc_renamed.md", test_dir);
    let result_content = fs::read_to_string(&renamed_path).unwrap();
    assert!(result_content.contains("# Title"));
    assert!(result_content.contains("**bold**"));
    assert!(result_content.contains("- item 1"));

    cleanup_test_env(&test_dir);
}

// ============= Rename with references =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_external_references() {
    let test_dir = create_test_env("ext_refs");

    let source = format!("{}/original.md", test_dir);
    create_test_file(&source, "# Original");

    let ref_file = format!("{}/index.md", test_dir);
    create_test_file(&ref_file, "See [original doc](original.md) for details.");

    rename_file(&source, "updated.md", &test_dir).unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("updated.md"));
    assert!(!ref_content.contains("original.md"));

    cleanup_test_env(&test_dir);
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_multiple_external_references() {
    let test_dir = create_test_env("multi_ext_refs");

    let source = format!("{}/target.md", test_dir);
    create_test_file(&source, "# Target");

    let ref1 = format!("{}/ref1.md", test_dir);
    let ref2 = format!("{}/ref2.md", test_dir);
    let ref3 = format!("{}/sub/ref3.md", test_dir);
    create_test_file(&ref1, "[Link](target.md)");
    create_test_file(&ref2, "[Another](target.md)");
    create_test_file(&ref3, "[Deep](../target.md)");

    rename_file(&source, "new_target.md", &test_dir).unwrap();

    let ref1_content = fs::read_to_string(&ref1).unwrap();
    let ref2_content = fs::read_to_string(&ref2).unwrap();
    let ref3_content = fs::read_to_string(&ref3).unwrap();

    assert!(ref1_content.contains("new_target.md"));
    assert!(ref2_content.contains("new_target.md"));
    assert!(ref3_content.contains("new_target.md"));

    cleanup_test_env(&test_dir);
}

// ============= Rename with internal links =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_self_reference() {
    let test_dir = create_test_env("self_ref");

    let source = format!("{}/page.md", test_dir);
    create_test_file(&source, "[Self link](page.md)");

    rename_file(&source, "page_v2.md", &test_dir).unwrap();

    let renamed_path = format!("{}/page_v2.md", test_dir);
    let content = fs::read_to_string(&renamed_path).unwrap();
    assert!(content.contains("page_v2.md"));
    assert!(!content.contains("page.md"));

    cleanup_test_env(&test_dir);
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_internal_links_to_other_files() {
    let test_dir = create_test_env("internal_links");

    let other = format!("{}/other.md", test_dir);
    create_test_file(&other, "# Other");

    let source = format!("{}/source.md", test_dir);
    create_test_file(&source, "[Other file](other.md)");

    rename_file(&source, "source_v2.md", &test_dir).unwrap();

    let renamed_path = format!("{}/source_v2.md", test_dir);
    let content = fs::read_to_string(&renamed_path).unwrap();
    // Renamed in same directory, so link to other.md should remain unchanged
    assert!(content.contains("other.md"));

    cleanup_test_env(&test_dir);
}

// ============= Rename in subdirectory =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_in_subdirectory() {
    let test_dir = create_test_env("subdir");

    let source = format!("{}/sub/deep.md", test_dir);
    create_test_file(&source, "# Deep file");

    let ref_file = format!("{}/root.md", test_dir);
    create_test_file(&ref_file, "[Deep](sub/deep.md)");

    rename_file(&source, "shallow.md", &test_dir).unwrap();

    assert!(Path::new(&format!("{}/sub/shallow.md", test_dir)).exists());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("sub/shallow.md"));
    assert!(!ref_content.contains("sub/deep.md"));

    cleanup_test_env(&test_dir);
}

// ============= Error cases =============

#[test]
fn test_rename_nonexistent_file() {
    let test_dir = create_test_env("nonexistent");

    let result = rename_file(&format!("{}/ghost.md", test_dir), "new.md", &test_dir);
    assert!(result.is_err());

    cleanup_test_env(&test_dir);
}
