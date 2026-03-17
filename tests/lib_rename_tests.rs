use mdref::rename;
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

    let result = rename(&source, "renamed.md", temp_dir.path(), false);

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

    rename(&source, "doc_renamed.md", temp_dir.path(), false).unwrap();

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

    rename(&source, "updated.md", temp_dir.path(), false).unwrap();

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

    rename(&source, "new_target.md", temp_dir.path(), false).unwrap();

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

    rename(&source, "page_v2.md", temp_dir.path(), false).unwrap();

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

    rename(&source, "source_v2.md", temp_dir.path(), false).unwrap();

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

    rename(&source, "shallow.md", temp_dir.path(), false).unwrap();

    assert!(temp_dir.path().join("sub").join("shallow.md").exists());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("sub/shallow.md"));
    assert!(!ref_content.contains("sub/deep.md"));
}

// ============= Error cases =============

#[test]
fn test_rename_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();

    let result = rename(
        temp_dir.path().join("ghost.md"),
        "new.md",
        temp_dir.path(),
        false,
    );
    assert!(result.is_err());
}

// ============= Unicode rename tests =============

/// Test renaming file with Chinese name to another Chinese name.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_chinese_filename() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("旧名称.md");
    write_file(&source, "# 中文文档");

    let result = rename(&source, "新名称.md", temp_dir.path(), false);

    assert!(result.is_ok());
    assert!(!source.exists());
    assert!(temp_dir.path().join("新名称.md").exists());
}

/// Test renaming ASCII file to Unicode filename.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_ascii_to_unicode() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("document.md");
    write_file(&source, "# Document");

    let result = rename(&source, "文档.md", temp_dir.path(), false);

    assert!(result.is_ok());
    assert!(temp_dir.path().join("文档.md").exists());
}

/// Test renaming Unicode file updates external references correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_unicode_updates_references() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("原始文档.md");
    write_file(&source, "# 原始文档");

    let ref_file = temp_dir.path().join("索引.md");
    write_file(&ref_file, "参考 [原始文档](原始文档.md) 获取更多信息。");

    rename(&source, "更新文档.md", temp_dir.path(), false).unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("更新文档.md"));
    assert!(!ref_content.contains("原始文档.md"));
}

/// Test renaming file with Japanese name.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_japanese_filename() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("旧文件.md");
    write_file(&source, "# ドキュメント");

    let result = rename(&source, "新文件.md", temp_dir.path(), false);

    assert!(result.is_ok());
    assert!(temp_dir.path().join("新文件.md").exists());
}

/// Test renaming file with emoji in name.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_emoji_filename() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("📝笔记.md");
    write_file(&source, "# Notes");

    let result = rename(&source, "📚文档.md", temp_dir.path(), false);

    assert!(result.is_ok());
    assert!(temp_dir.path().join("📚文档.md").exists());
}
