use std::fs;

use mdref::{NoopProgress, rename};
use rstest::rstest;

mod common;

use common::{
    fixture_multi_file_reference, fixture_single_file_reference, fixture_unicode_paths, read_file,
    temp_dir, write_file,
};

#[allow(clippy::unwrap_used)]
fn is_case_insensitive_filesystem(root: &std::path::Path) -> bool {
    let mixed_case = root.join("CaseProbe.md");
    fs::write(&mixed_case, "# probe").unwrap();
    root.join("CASEPROBE.md").exists()
}

// Library tests for `rename` cover the rename semantics and reference updates.
// CLI tests avoid duplicating these cases unless process behavior must be verified.

// ============= Rename core behavior tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_same_directory_moves_file_to_new_name() {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join("source.md");
    write_file(&source, "# Source File\n\nSome content.");

    let result = rename(&source, "renamed.md", temp_dir.path(), false, &NoopProgress);

    assert!(result.is_ok());
    assert!(!source.exists());
    assert!(temp_dir.path().join("renamed.md").exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_content() {
    let temp_dir = temp_dir();
    let content = "# Title\n\nParagraph with **bold** and *italic*.\n\n- item 1\n- item 2";
    let source = temp_dir.path().join("doc.md");
    write_file(&source, content);

    rename(
        &source,
        "doc_renamed.md",
        temp_dir.path(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let renamed_path = temp_dir.path().join("doc_renamed.md");
    let result_content = read_file(&renamed_path);
    assert!(result_content.contains("# Title"));
    assert!(result_content.contains("**bold**"));
    assert!(result_content.contains("- item 1"));
}

// ============= Rename with references =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_external_references() {
    let fixture = fixture_single_file_reference();
    let source = fixture.target;
    let ref_file = fixture.reference;

    rename(&source, "updated.md", &fixture.root, false, &NoopProgress).unwrap();

    let ref_content = read_file(&ref_file);
    assert!(ref_content.contains("updated.md"));
    assert!(!ref_content.contains("target.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_multiple_external_references() {
    let fixture = fixture_multi_file_reference();
    let source = fixture.target;

    rename(
        &source,
        "new_target.md",
        &fixture.root,
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref1_content = read_file(&fixture.primary_reference);
    let ref2_content = read_file(&fixture.secondary_reference);
    let ref3_content = read_file(&fixture.nested_reference);

    assert!(ref1_content.contains("new_target.md"));
    assert!(ref2_content.contains("new_target.md"));
    assert!(ref3_content.contains("new_target.md"));
}

// ============= Rename with internal links =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_updates_self_reference() {
    let temp_dir = temp_dir();

    let source = temp_dir.path().join("page.md");
    write_file(&source, "[Self link](page.md)");

    rename(&source, "page_v2.md", temp_dir.path(), false, &NoopProgress).unwrap();

    let renamed_path = temp_dir.path().join("page_v2.md");
    let content = read_file(&renamed_path);
    assert!(content.contains("page_v2.md"));
    assert!(!content.contains("page.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_preserves_internal_links_to_other_files() {
    let temp_dir = temp_dir();

    let other = temp_dir.path().join("other.md");
    write_file(&other, "# Other");

    let source = temp_dir.path().join("source.md");
    write_file(&source, "[Other file](other.md)");

    rename(
        &source,
        "source_v2.md",
        temp_dir.path(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let renamed_path = temp_dir.path().join("source_v2.md");
    let content = read_file(&renamed_path);
    // Renamed in same directory, so link to other.md should remain unchanged
    assert!(content.contains("other.md"));
}

// ============= Rename in subdirectory =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_in_subdirectory() {
    let temp_dir = temp_dir();

    let source = temp_dir.path().join("sub").join("deep.md");
    write_file(&source, "# Deep file");

    let ref_file = temp_dir.path().join("root.md");
    write_file(&ref_file, "[Deep](sub/deep.md)");

    rename(&source, "shallow.md", temp_dir.path(), false, &NoopProgress).unwrap();

    assert!(temp_dir.path().join("sub").join("shallow.md").exists());

    let ref_content = read_file(&ref_file);
    assert!(ref_content.contains("sub/shallow.md"));
    assert!(!ref_content.contains("sub/deep.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_case_only_name_on_case_insensitive_filesystem_updates_file_and_references() {
    let temp_dir = temp_dir();
    if !is_case_insensitive_filesystem(temp_dir.path()) {
        return;
    }

    let source = temp_dir.path().join("Readme.md");
    write_file(&source, "[Self](Readme.md)");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[Guide](Readme.md)");

    rename(&source, "README.md", temp_dir.path(), false, &NoopProgress).unwrap();

    let renamed = temp_dir.path().join("README.md");
    assert!(renamed.exists());
    let file_names: Vec<String> = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(file_names.iter().any(|name| name == "README.md"));
    assert!(!file_names.iter().any(|name| name == "Readme.md"));

    let renamed_content = read_file(&renamed);
    assert!(renamed_content.contains("README.md"));
    assert!(!renamed_content.contains("Readme.md"));

    let ref_content = read_file(&ref_file);
    assert!(ref_content.contains("README.md"));
    assert!(!ref_content.contains("Readme.md"));
}

// ============= Error cases =============

#[test]
fn test_rename_nonexistent_file() {
    let temp_dir = temp_dir();

    let result = rename(
        temp_dir.path().join("ghost.md"),
        "new.md",
        temp_dir.path(),
        false,
        &NoopProgress,
    );
    assert!(result.is_err());
}

// ============= Unicode rename tests =============

/// Test renaming files with Unicode filenames (Chinese, Japanese, emoji).
#[rstest]
#[case::chinese("旧名称.md", "新名称.md", "# 中文文档")]
#[case::japanese("旧文件.md", "新文件.md", "# ドキュメント")]
#[case::emoji("📝笔记.md", "📚文档.md", "# Notes")]
#[case::ascii_to_unicode("document.md", "文档.md", "# Document")]
#[allow(clippy::unwrap_used)]
fn test_rename_unicode_filename(
    #[case] old_name: &str,
    #[case] new_name: &str,
    #[case] content: &str,
) {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join(old_name);
    write_file(&source, content);

    let result = rename(&source, new_name, temp_dir.path(), false, &NoopProgress);

    assert!(result.is_ok());
    assert!(!source.exists());
    assert!(temp_dir.path().join(new_name).exists());
}

/// Test renaming Unicode file updates external references correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_rename_unicode_updates_references() {
    let fixture = fixture_unicode_paths();

    rename(
        &fixture.source,
        "更新文档.md",
        &fixture.root,
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref_content = read_file(&fixture.reference);
    assert!(ref_content.contains("更新文档.md"));
    assert!(!ref_content.contains("原始文档.md"));
}
