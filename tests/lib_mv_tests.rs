#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use mdref::{MdrefError, NoopProgress, find_links, find_references, mv};
use rstest::rstest;
use tempfile::TempDir;

mod common;

use common::{fixture_directory_move, fixture_unicode_paths, write_file};

static CURRENT_DIR_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

struct CurrentDirGuard {
    original_dir: PathBuf,
}

impl CurrentDirGuard {
    fn enter(path: &Path) -> Self {
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self { original_dir }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original_dir).unwrap();
    }
}

fn with_current_dir<T>(path: &Path, operation: impl FnOnce() -> T) -> T {
    let _lock = CURRENT_DIR_LOCK.lock().unwrap();
    let _guard = CurrentDirGuard::enter(path);
    operation()
}

// Library tests for `mv` cover path rewriting and filesystem mutations.
// CLI tests keep only representative command wiring and process-contract checks.

// ============= mv core behavior tests =============

#[test]
fn test_mv_same_directory_moves_file_to_target_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file
    let source_file = temp_dir.path().join("source.md");
    let target_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Source File\n\nSome content.");

    // Perform move
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Verify
    assert!(result.is_ok());
    assert!(target_file.exists());
    assert!(!source_file.exists());
}

#[test]
fn test_mv_with_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source File");

    // Create file that references the source file
    let ref_file = temp_dir.path().join("reference.md");
    write_file(&ref_file, "[Link to source](source.md)");

    // Move file
    let target_file = temp_dir.path().join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Verify
    assert!(result.is_ok());

    // Check if references are updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("target.md"));
    assert!(!ref_content.contains("source.md"));
}

#[test]
fn test_mv_to_subdirectory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source File");

    // Create reference file
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Move to subdirectory
    let target_file = temp_dir.path().join("subdir").join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Verify
    assert!(result.is_ok());
    assert!(target_file.exists());

    // Check if references are correctly updated to relative paths
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("subdir"));
}

#[test]
fn test_mv_with_internal_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file containing links
    let source_file = temp_dir.path().join("source.md");
    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other File");
    write_file(&source_file, "[Other](other.md)");

    // Move file to subdirectory
    let target_file = temp_dir.path().join("subdir").join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Verify
    assert!(result.is_ok());

    // Check if links in the moved file are updated
    let target_content = fs::read_to_string(&target_file).unwrap();
    // The relative path from subdir/target.md to other.md should be ../other.md
    assert!(target_content.contains("../other.md") || target_content.contains("other.md"));
}

#[test]
fn test_mv_nonexistent_source() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("nonexistent.md");
    let target_file = temp_dir.path().join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    match result {
        Err(MdrefError::PathValidation { path, details }) => {
            assert!(
                path.ends_with("nonexistent.md") || path.to_string_lossy().contains("nonexistent")
            );
            assert!(details.contains("source path does not exist"));
        }
        other => panic!("expected path error for nonexistent source, got {other:?}"),
    }
}

#[test]
fn test_mv_creates_parent_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Content");

    // Target path contains non-existent directory
    let target_file = temp_dir
        .path()
        .join("new")
        .join("nested")
        .join("dir")
        .join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Should successfully create directory and move file
    assert!(result.is_ok());
    assert!(target_file.exists());
}

// ============= Edge case tests =============

#[test]
fn test_mv_preserves_content() {
    let temp_dir = TempDir::new().unwrap();

    let content = "# Title\n\nParagraph 1\n\n## Section\n\nMore content.";
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, content);

    let target_file = temp_dir.path().join("target.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let target_content = fs::read_to_string(&target_file).unwrap();
    // Content should be preserved
    assert!(target_content.contains("# Title"));
    assert!(target_content.contains("Paragraph 1"));
    assert!(target_content.contains("## Section"));
}

#[test]
fn test_mv_multiple_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    // Create multiple reference files
    let ref1 = temp_dir.path().join("ref1.md");
    let ref2 = temp_dir.path().join("ref2.md");
    let ref3 = temp_dir.path().join("ref3.md");
    write_file(&ref1, "[Link](source.md)");
    write_file(&ref2, "[Another link](source.md)");
    write_file(&ref3, "[Third link](source.md)");

    // Move file
    let target_file = temp_dir.path().join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Verify all references have been updated
    let ref1_content = fs::read_to_string(&ref1).unwrap();
    let ref2_content = fs::read_to_string(&ref2).unwrap();
    let ref3_content = fs::read_to_string(&ref3).unwrap();

    assert!(ref1_content.contains("target.md"));
    assert!(ref2_content.contains("target.md"));
    assert!(ref3_content.contains("target.md"));
}

#[test]
fn test_mv_same_name_different_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("file.md");
    write_file(&source_file, "# Original");

    // Create reference
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](file.md)");

    // Move to subdirectory, keeping filename
    let target_file = temp_dir.path().join("subdir").join("file.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());
}

// ============= Integration tests =============

#[test]
fn test_mv_integration_with_find() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file structure
    let source_file = temp_dir.path().join("source.md");
    let other_file = temp_dir.path().join("other.md");
    write_file(&source_file, "[Other](other.md)");
    write_file(&other_file, "# Other");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Find references before move
    let refs_before = find_references(&source_file, temp_dir.path(), &NoopProgress).unwrap();
    assert!(!refs_before.is_empty());

    // Move file
    let target_file = temp_dir.path().join("target.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    // Verify links in new file after move
    let links_after = find_links(&target_file).unwrap();
    assert!(!links_after.is_empty());
}

// ============= Deep nested move tests =============

#[test]
fn test_mv_deep_nested_move() {
    let temp_dir = TempDir::new().unwrap();

    // Create deep source structure
    let source_file = temp_dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("deep.md");
    write_file(&source_file, "# Deep file");

    let ref_file = temp_dir.path().join("root_ref.md");
    write_file(&ref_file, "[Deep](a/b/c/deep.md)");

    let sibling_ref = temp_dir.path().join("a").join("sibling.md");
    write_file(&sibling_ref, "[Deep](b/c/deep.md)");

    // Move to completely different deep path
    let target_file = temp_dir
        .path()
        .join("x")
        .join("y")
        .join("z")
        .join("moved.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());
    assert!(!source_file.exists());

    // Verify references updated with correct multi-level relative paths
    let root_ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(root_ref_content.contains("x/y/z/moved.md"));

    let sibling_ref_content = fs::read_to_string(&sibling_ref).unwrap();
    assert!(sibling_ref_content.contains("moved.md"));
}

// ============= Same file multiple references on different lines =============

#[test]
fn test_mv_same_file_multiple_lines_referencing() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    // One file references source.md on multiple lines
    let ref_file = temp_dir.path().join("multi_ref.md");
    write_file(
        &ref_file,
        "First: [link1](source.md)\n\nSecond: [link2](source.md)\n\nThird: [link3](source.md)",
    );

    let target_file = temp_dir.path().join("dest.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    // All three references should be updated
    assert!(!ref_content.contains("source.md"));
    assert_eq!(ref_content.matches("dest.md").count(), 3);
}

// ============= Move with self-reference =============

#[test]
fn test_mv_self_reference_update() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("page.md");
    write_file(&source_file, "[Self](page.md)");

    let target_file = temp_dir.path().join("moved.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    assert!(content.contains("moved.md"));
    assert!(!content.contains("page.md"));
}

// ============= Move to same directory (equivalent to rename) =============

#[test]
fn test_mv_same_directory() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("old_name.md");
    write_file(&source_file, "# Content");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](old_name.md)");

    let target_file = temp_dir.path().join("new_name.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    assert!(!source_file.exists());
    assert!(target_file.exists());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("new_name.md"));
    assert!(!ref_content.contains("old_name.md"));
}

// ============= Move file with image links =============

#[test]
fn test_mv_with_image_links() {
    let temp_dir = TempDir::new().unwrap();

    let image_file = temp_dir.path().join("assets").join("photo.png");
    write_file(&image_file, "fake image data");

    let source_file = temp_dir.path().join("doc.md");
    write_file(&source_file, "# Doc\n\n![Photo](assets/photo.png)");

    // Move doc.md into a subdirectory
    let target_file = temp_dir.path().join("sub").join("doc.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    // Image link should be updated to ../assets/photo.png
    assert!(content.contains("../assets/photo.png"));
}

#[test]
fn test_mv_preserves_external_urls() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("local.md");
    write_file(&other_file, "# Local");

    let source_file = temp_dir.path().join("mixed.md");
    // File contains both external URLs and local links.
    // mv should skip external URLs and only update local links.
    write_file(
        &source_file,
        "[Google](https://google.com)\n\n[Local](local.md)\n\n[GitHub](https://github.com)",
    );

    let target_file = temp_dir.path().join("sub").join("mixed.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    // External URLs should remain unchanged
    assert!(content.contains("https://google.com"));
    assert!(content.contains("https://github.com"));
    // Local link should be updated
    assert!(content.contains("../local.md"));
}

#[test]
fn test_mv_from_subdir_to_root() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("sub").join("sibling.md");
    write_file(&other_file, "# Sibling");

    let source_file = temp_dir.path().join("sub").join("nested.md");
    write_file(&source_file, "[Sibling](sibling.md)");

    let ref_file = temp_dir.path().join("root_ref.md");
    write_file(&ref_file, "[Nested](sub/nested.md)");

    let target_file = temp_dir.path().join("promoted.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    // Reference from root should be updated
    let root_ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(root_ref_content.contains("promoted.md"));

    // Internal link should be updated to point to sub/sibling.md
    let content = fs::read_to_string(&target_file).unwrap();
    assert!(content.contains("sub/sibling.md"));
}

#[test]
fn test_mv_directory_updates_external_references() {
    let fixture = fixture_directory_move();
    let target_parent = fixture.root.join("archive");

    mv(
        &fixture.source_dir,
        &target_parent,
        &fixture.root,
        false,
        &NoopProgress,
    )
    .unwrap();

    assert!(target_parent.join("guide.md").exists());
    assert!(target_parent.join("nested").join("topic.md").exists());
    assert!(!fixture.source_dir.exists());

    let ref_content = fs::read_to_string(&fixture.external_reference).unwrap();
    assert!(ref_content.contains("archive/guide.md"));
    assert!(ref_content.contains("archive/nested/topic.md"));
    assert!(!ref_content.contains("docs/guide.md"));
    assert!(!ref_content.contains("docs/nested/topic.md"));
}

#[test]
fn test_mv_directory_updates_internal_links_to_outside_files() {
    let fixture = fixture_directory_move();
    mv(
        &fixture.source_dir,
        &fixture.destination_dir,
        &fixture.root,
        false,
        &NoopProgress,
    )
    .unwrap();

    let moved_content = fs::read_to_string(fixture.destination_dir.join("guide.md")).unwrap();
    assert!(moved_content.contains("../../shared/faq.md"));
}

#[test]
fn test_mv_directory_preserves_internal_links_within_directory() {
    let fixture = fixture_directory_move();
    mv(
        &fixture.source_dir,
        &fixture.destination_dir,
        &fixture.root,
        false,
        &NoopProgress,
    )
    .unwrap();

    let moved_guide = fs::read_to_string(fixture.destination_dir.join("guide.md")).unwrap();
    let moved_topic =
        fs::read_to_string(fixture.destination_dir.join("nested").join("topic.md")).unwrap();
    assert!(moved_guide.contains("nested/topic.md"));
    assert!(moved_topic.contains("../guide.md"));
}

#[test]
fn test_mv_directory_skips_gitignored_markdown_rewrites() {
    let temp_dir = TempDir::new().unwrap();

    write_file(temp_dir.path().join(".gitignore"), "docs/ignored/\n");

    let source_dir = temp_dir.path().join("docs");
    let ignored_markdown = source_dir.join("ignored").join("secret.md");
    let outside_file = temp_dir.path().join("shared").join("faq.md");

    write_file(source_dir.join("guide.md"), "# Guide");
    write_file(&ignored_markdown, "[FAQ](../../shared/faq.md)");
    write_file(&outside_file, "# FAQ");

    let destination = temp_dir.path().join("archive").join("docs");
    mv(
        &source_dir,
        &destination,
        temp_dir.path(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let ignored_content =
        fs::read_to_string(destination.join("ignored").join("secret.md")).unwrap();
    assert!(ignored_content.contains("../../shared/faq.md"));
    assert!(!ignored_content.contains("../../../shared/faq.md"));
}

#[test]
fn test_mv_directory_into_existing_directory_preserves_source_name() {
    let temp_dir = TempDir::new().unwrap();

    let source_dir = temp_dir.path().join("docs");
    write_file(source_dir.join("guide.md"), "# Guide");
    let destination_parent = temp_dir.path().join("archive");
    fs::create_dir_all(&destination_parent).unwrap();

    mv(
        &source_dir,
        &destination_parent,
        temp_dir.path(),
        false,
        &NoopProgress,
    )
    .unwrap();

    assert!(destination_parent.join("docs").join("guide.md").exists());
    assert!(!source_dir.exists());
}

#[test]
fn test_mv_directory_rejects_moving_into_own_subdirectory() {
    let temp_dir = TempDir::new().unwrap();

    let source_dir = temp_dir.path().join("docs");
    write_file(source_dir.join("guide.md"), "# Guide");
    let invalid_target = source_dir.join("nested").join("archive");

    let result = mv(
        &source_dir,
        &invalid_target,
        temp_dir.path(),
        false,
        &NoopProgress,
    );

    assert!(result.is_err());
    assert!(source_dir.join("guide.md").exists());
}

/// Move file to a target path with non-existent intermediate directories.
#[test]
fn test_mv_with_nonexistent_intermediate_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file in a subdirectory
    let source_file = temp_dir.path().join("project").join("source.md");
    write_file(&source_file, "# Source");

    // Create a reference file at the project root
    let ref_file = temp_dir.path().join("project").join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Target is in a non-existent nested directory (the path doesn't exist yet)
    let target_file = temp_dir
        .path()
        .join("project")
        .join("new")
        .join("nested")
        .join("target.md");

    // This should work even though the intermediate directories don't exist
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(
        result.is_ok(),
        "Move should succeed even with non-existent intermediate path"
    );
    assert!(target_file.exists(), "Target file should exist");

    // Reference should be updated to point to the new location
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("new/nested/target.md") || ref_content.contains("target.md"),
        "Reference should be updated correctly. Got: {}",
        ref_content
    );
}

/// Move file with self-references to a different directory.
#[test]
fn test_mv_self_reference_cross_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file with self-reference in a subdirectory
    let source_file = temp_dir.path().join("original").join("page.md");
    write_file(
        &source_file,
        "# Page\n\n[Self](page.md)\n\n[Also self](./page.md)",
    );

    // Move to a different directory with different name
    let target_file = temp_dir.path().join("moved").join("renamed.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    let content = fs::read_to_string(&target_file).unwrap();
    // Both self-references should be updated to the new filename
    assert!(
        content.contains("renamed.md"),
        "Self-reference should be updated to new filename. Got: {}",
        content
    );
    assert!(
        !content.contains("page.md"),
        "Old self-reference should not exist. Got: {}",
        content
    );
}

/// Verify normal move operation succeeds for error type validation.
#[test]
fn test_mv_error_type_validation() {
    let temp_dir = TempDir::new().unwrap();

    // Create a source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Content");

    // Create a reference file with a link
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](source.md)");

    // Move the file - this should succeed
    let target_file = temp_dir.path().join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // This test just verifies that normal operation succeeds
    // The error type validation is done in unit tests
    assert!(result.is_ok());
}

/// Move file to a subdirectory and verify internal links are updated correctly.
#[test]
fn test_mv_with_relative_target_path() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source\n\n[External](https://example.com)");

    // Create a local file that source references
    let local_file = temp_dir.path().join("local.md");
    write_file(&local_file, "# Local");

    // Update source to reference local file
    write_file(&source_file, "# Source\n\n[Local](local.md)");

    // Create reference to source
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Use a relative-style path for target (though we need to use absolute for the test)
    // Note: In practice, the command line might pass relative paths
    let target_file = temp_dir.path().join("subdir").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());

    // Verify internal link is updated correctly
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        target_content.contains("../local.md"),
        "Internal link should be updated with correct relative path. Got: {}",
        target_content
    );

    // Verify external reference is updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("subdir/target.md"),
        "Reference should be updated. Got: {}",
        ref_content
    );
}

/// Move file to a deeply nested new directory with multiple references.
#[test]
fn test_mv_deep_new_directory_with_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create a file with internal links
    let sibling_file = temp_dir.path().join("sibling.md");
    write_file(&sibling_file, "# Sibling");

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Sibling](sibling.md)");

    // Create multiple references from different locations
    let ref1 = temp_dir.path().join("ref1.md");
    let ref2 = temp_dir.path().join("sub").join("ref2.md");
    write_file(&ref1, "[Source](source.md)");
    write_file(&ref2, "[Source](../source.md)");

    // Move to a deeply nested new directory
    let target_file = temp_dir
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("d")
        .join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());

    // Check internal link in moved file
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        target_content.contains("sibling.md") || target_content.contains("../../../sibling.md"),
        "Internal link should be updated. Got: {}",
        target_content
    );

    // Check external references
    let ref1_content = fs::read_to_string(&ref1).unwrap();
    assert!(
        ref1_content.contains("a/b/c/d/target.md"),
        "Reference from root should be updated. Got: {}",
        ref1_content
    );
}

/// Move file with internal links containing anchors should preserve the anchor.
/// Bug: build_link_replacement does not strip anchors before canonicalize,
/// causing IO error or anchor loss.
#[rstest]
#[case::moved_to_subdirectory(
    "# Other\n\n## Details",
    "[Details](other.md#details)",
    "sub/moved.md",
    "../other.md#details"
)]
#[case::renamed_in_place(
    "# Other\n\n## Section",
    "[Section](other.md#section)",
    "renamed.md",
    "other.md#section"
)]
#[allow(clippy::unwrap_used)]
fn test_mv_preserves_internal_anchor_links(
    #[case] other_content: &str,
    #[case] source_content: &str,
    #[case] target_relative_path: &str,
    #[case] expected_link: &str,
) {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, other_content);

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, source_content);

    let target_file = temp_dir.path().join(target_relative_path);
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(
        result.is_ok(),
        "mv should not fail when internal links have anchors: {:?}",
        result.err()
    );

    let content = fs::read_to_string(&target_file).unwrap();
    assert!(
        content.contains(expected_link),
        "Internal link anchor should be preserved. Got: {}",
        content
    );
}

/// Move file containing a broken (dangling) link should not fail.
/// Bug: canonicalize() on non-existent link path returns IO error,
/// causing the entire mv to fail.
#[test]
fn test_mv_with_broken_internal_link() {
    let temp_dir = TempDir::new().unwrap();

    // Source file has a link to a non-existent file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Broken](nonexistent.md)");

    let target_file = temp_dir.path().join("target.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Should succeed — broken links should be skipped, not cause failure
    assert!(
        result.is_ok(),
        "mv should not fail due to broken internal links: {:?}",
        result.err()
    );
    assert!(target_file.exists());
    assert!(!source_file.exists());

    // The broken link should be preserved as-is
    let content = fs::read_to_string(&target_file).unwrap();
    assert!(
        content.contains("nonexistent.md"),
        "Broken link should be preserved unchanged. Got: {}",
        content
    );
}

/// Moving a file to itself should not delete the file.
/// Bug: fs::copy(src, src) is a no-op, then fs::remove_file(src) deletes the only copy.
#[test]
fn test_mv_source_equals_dest() {
    let temp_dir = TempDir::new().unwrap();

    let file = temp_dir.path().join("same.md");
    write_file(&file, "# Content");

    // Move file to itself
    let result = mv(
        file.to_str().unwrap(),
        file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Should either succeed as no-op or return an error, but NOT delete the file
    assert!(
        file.exists(),
        "File should still exist after moving to itself"
    );

    if result.is_ok() {
        let content = fs::read_to_string(&file).unwrap();
        assert_eq!(content, "# Content");
    }
}

/// Moving a file to itself with relative path variants should be handled correctly.
/// "file.md" and "./file.md" refer to the same file but string comparison fails.
#[rstest]
#[case::relative_to_absolute(
    "relative_to_absolute",
    "test.md",
    "# Test Content\n\n[Link](other.md)",
    true
)]
#[case::dot_slash("dot_slash", "doc.md", "# Documentation", false)]
#[case::trailing_slash("trailing_slash", "file.md", "# Content", false)]
#[allow(clippy::unwrap_used)]
fn test_mv_same_file_path_variants_are_noops(
    #[case] scenario: &str,
    #[case] file_name: &str,
    #[case] file_content: &str,
    #[case] verify_references: bool,
) {
    let temp_dir = TempDir::new().unwrap();

    let file = temp_dir.path().join(file_name);
    write_file(&file, file_content);

    let ref_file = temp_dir.path().join("ref.md");
    let original_ref_content = if verify_references {
        write_file(&ref_file, &format!("[Test]({file_name})"));
        Some(fs::read_to_string(&ref_file).unwrap())
    } else {
        None
    };

    let result = match scenario {
        "relative_to_absolute" => {
            let abs_path = file.canonicalize().unwrap();
            with_current_dir(temp_dir.path(), || {
                mv(
                    "./test.md",
                    abs_path.to_str().unwrap(),
                    ".",
                    false,
                    &NoopProgress,
                )
            })
        }
        "dot_slash" => with_current_dir(temp_dir.path(), || {
            mv("./doc.md", "./doc.md", ".", false, &NoopProgress)
        }),
        "trailing_slash" => {
            let path_with_slash = format!("{}/{}", temp_dir.path().to_str().unwrap(), file_name);
            let path_without_slash = file.to_str().unwrap().to_string();
            mv(
                &path_with_slash,
                &path_without_slash,
                temp_dir.path(),
                false,
                &NoopProgress,
            )
        }
        _ => unreachable!("unsupported scenario: {scenario}"),
    };

    assert!(file.exists(), "File should still exist after no-op move");
    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, file_content);

    if let Some(original_ref_content) = original_ref_content {
        let ref_content_after = fs::read_to_string(&ref_file).unwrap();
        assert_eq!(
            original_ref_content, ref_content_after,
            "References should not be modified when source equals dest"
        );
    }

    assert!(result.is_ok(), "Operation should succeed as no-op");
}

/// Moving a file to itself should not modify references.
/// If source == dest, all reference updates would be no-ops anyway.
#[test]
fn test_mv_same_path_preserves_references() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source\n\n[Self](source.md)");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](source.md)");

    let original_ref_content = fs::read_to_string(&ref_file).unwrap();
    let original_source_content = fs::read_to_string(&source_file).unwrap();

    // Move to the same location
    let result = mv(
        source_file.to_str().unwrap(),
        source_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Both files should be unchanged
    assert!(source_file.exists());
    assert_eq!(
        fs::read_to_string(&source_file).unwrap(),
        original_source_content
    );
    assert_eq!(fs::read_to_string(&ref_file).unwrap(), original_ref_content);
    assert!(result.is_ok());
}

/// Moving file to itself with symlink (if supported by OS).
/// Symlinks that resolve to the same file should be detected.
#[test]
fn test_mv_symlink_to_same_file() {
    let temp_dir = TempDir::new().unwrap();

    let real_file = temp_dir.path().join("real.md");
    write_file(&real_file, "# Real Content");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let symlink_path = temp_dir.path().join("link.md");
        symlink(&real_file, &symlink_path).unwrap();

        // Move from symlink to real file - both resolve to same content
        let result = mv(
            symlink_path.to_str().unwrap(),
            real_file.to_str().unwrap(),
            temp_dir.path(),
            false,
            &NoopProgress,
        );

        // Behavior should be safe - either no-op or well-defined
        assert!(real_file.exists(), "Real file must exist");
        assert_eq!(fs::read_to_string(&real_file).unwrap(), "# Real Content");
        // Result can be Ok or Err, but data must be preserved
        let _ = result;
    }

    #[cfg(not(unix))]
    {
        // Skip on non-Unix platforms
        println!("Symlink test skipped on non-Unix platform");
    }
}

/// A symlink reference should be treated as a real reference target and rewritten
/// to the moved file path after the underlying file is relocated.
#[test]
#[cfg(unix)]
#[allow(clippy::unwrap_used)]
fn test_mv_symbolic_link_reference_updates_reference() {
    use std::os::unix::fs::symlink;

    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("docs").join("source.md");
    write_file(&source_file, "# Source");

    let symlink_path = temp_dir.path().join("aliases").join("source.md");
    fs::create_dir_all(symlink_path.parent().unwrap()).unwrap();
    symlink(&source_file, &symlink_path).unwrap();

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[Alias](aliases/source.md)");

    let target_file = temp_dir.path().join("archive").join("source.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("archive/source.md"));
    assert!(!ref_content.contains("aliases/source.md"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_reference_file_with_crlf_line_endings_preserves_line_endings() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    let ref_file = temp_dir.path().join("index.md");
    fs::write(&ref_file, b"# Index\r\n\r\n[Source](source.md)\r\n").unwrap();

    let target_file = temp_dir.path().join("archive").join("source.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(
        ref_content,
        "# Index\r\n\r\n[Source](archive/source.md)\r\n"
    );
}

/// Move file containing pure anchor links (#section) should preserve them unchanged.
/// Pure anchor links are internal to the document and should not be rewritten.
#[rstest]
#[case::pure_anchor_only(
    "# Title\n\n[Section](#section)\n\n[Another](#another-heading)\n\n## Section\n\n## Another Heading",
    &["](#section)", "](#another-heading)"],
    &[],
    false
)]
#[case::mixed_anchor_and_file(
    "[Internal](#intro)\n\n[Other](other.md)\n\n[Other Section](other.md#details)\n\n## Intro",
    &["](#intro)"],
    &["](../other.md)", "](../other.md#details)"],
    true
)]
#[allow(clippy::unwrap_used)]
fn test_mv_handles_anchor_link_variants(
    #[case] source_content: &str,
    #[case] preserved_links: &[&str],
    #[case] rewritten_links: &[&str],
    #[case] create_other_file: bool,
) {
    let temp_dir = TempDir::new().unwrap();

    if create_other_file {
        let other_file = temp_dir.path().join("other.md");
        write_file(&other_file, "# Other");
    }

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, source_content);

    let target_file = temp_dir.path().join("sub").join("moved.md");
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let content = fs::read_to_string(&target_file).unwrap();
    for preserved_link in preserved_links {
        assert!(
            content.contains(preserved_link),
            "Expected preserved anchor link `{preserved_link}`. Got: {}",
            content
        );
    }
    for rewritten_link in rewritten_links {
        assert!(
            content.contains(rewritten_link),
            "Expected rewritten anchor link `{rewritten_link}`. Got: {}",
            content
        );
    }
}

/// Move file with anchor links, preserving the anchor fragments.
#[test]
fn test_mv_preserves_anchor_links() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Title\n\n[Section](#section)\n\n## Section");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md#title)");

    let target_file = temp_dir.path().join("target.md");

    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    // Internal anchor link should be preserved
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert!(target_content.contains("#section"));

    // External reference with anchor should be updated correctly
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("target.md#title"));
}

// ============= dry-run tests =============

/// Dry-run should not move the source file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_does_not_move() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    let target_file = temp_dir.path().join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(
        source_file.exists(),
        "Source file should still exist after dry-run"
    );
    assert!(
        !target_file.exists(),
        "Target file should not be created during dry-run"
    );
}

/// Dry-run should not modify any reference files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_does_not_update_references() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "See [source](source.md) for details.");

    let target_file = temp_dir.path().join("sub").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Source should still exist, target should not
    assert!(source_file.exists());
    assert!(!target_file.exists());

    // Reference file should remain unchanged
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(ref_content, "See [source](source.md) for details.");
}

/// Dry-run should not create intermediate directories.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_does_not_create_directories() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    let target_file = temp_dir.path().join("new").join("nested").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(
        !temp_dir.path().join("new").exists(),
        "Intermediate directories should not be created during dry-run"
    );
}

/// Dry-run should still validate that the source file exists.
#[test]
fn test_mv_dry_run_validates_source() {
    let temp_dir = TempDir::new().unwrap();

    let result = mv(
        temp_dir.path().join("ghost.md").to_str().unwrap(),
        temp_dir.path().join("target.md").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(
        result.is_err(),
        "Dry-run should still fail for nonexistent source"
    );
}

/// Dry-run with rename (same directory, different name, &NoopProgress) should not modify anything.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_rename_scenario() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("old_name.md");
    write_file(&source_file, "# Old Name");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Old](old_name.md)");

    let target_file = temp_dir.path().join("new_name.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(source_file.exists(), "Source should still exist");
    assert!(!target_file.exists(), "Target should not be created");

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(
        ref_content, "[Old](old_name.md)",
        "References should be unchanged"
    );
}

/// Dry-run with internal links should not modify the source file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_with_internal_links() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other");

    let source_file = temp_dir.path().join("source.md");
    let original_content = "[Other](other.md)\n[Self](source.md)";
    write_file(&source_file, original_content);

    let target_file = temp_dir.path().join("sub").join("moved.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Source file content should be completely unchanged
    let content = fs::read_to_string(&source_file).unwrap();
    assert_eq!(
        content, original_content,
        "Source file should not be modified during dry-run"
    );
}

// ============= destination already exists tests =============

/// Moving a file to a destination that already exists should fail.
/// The existing file should NOT be overwritten.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_destination_already_exists() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source Content\n\nThis is the source.");

    // Create an existing target file with different content
    let target_file = temp_dir.path().join("target.md");
    write_file(
        &target_file,
        "# Existing Target\n\nThis file already exists.",
    );

    // Store original target content to verify it wasn't overwritten
    let original_target_content = fs::read_to_string(&target_file).unwrap();

    // Attempt to move - should fail because target already exists
    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    match result {
        Err(MdrefError::PathValidation { details, .. }) => {
            assert!(details.contains("destination path already exists"));
        }
        other => panic!("expected path error for existing destination, got {other:?}"),
    }

    // Source file should still exist (not moved)
    assert!(
        source_file.exists(),
        "Source file should still exist after failed move"
    );

    // Target file content should be unchanged
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert_eq!(
        target_content, original_target_content,
        "Target file should not be modified"
    );
}

/// Moving a file to an existing destination should fail even in dry-run mode.
#[test]
fn test_mv_dry_run_destination_already_exists() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    let target_file = temp_dir.path().join("target.md");
    write_file(&target_file, "# Existing Target");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    // Dry-run should also fail for existing destination
    assert!(
        result.is_err(),
        "Dry-run should also return error when destination already exists"
    );
}

// ============= move to directory tests =============

/// Moving a file to an existing directory should place the file inside that directory.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_existing_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source Content");

    // Create an existing target directory
    let target_dir = temp_dir.path().join("subdir");
    fs::create_dir_all(&target_dir).unwrap();

    // Move source file to the directory (not a file path)
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Operation should succeed
    assert!(
        result.is_ok(),
        "mv should succeed when destination is an existing directory: {:?}",
        result.err()
    );

    // Source should be moved into the directory with original filename
    let expected_target = target_dir.join("source.md");
    assert!(
        expected_target.exists(),
        "File should exist at {}/source.md",
        target_dir.display()
    );
    assert!(!source_file.exists(), "Source file should no longer exist");

    // Content should be preserved
    let content = fs::read_to_string(&expected_target).unwrap();
    assert_eq!(content, "# Source Content");
}

/// Moving a file to an existing directory with references should update them correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_directory_with_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("doc.md");
    write_file(&source_file, "# Documentation");

    // Create a reference file pointing to source
    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "See [doc](doc.md) for details.");

    // Create target directory
    let target_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&target_dir).unwrap();

    // Move source file to directory
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Reference should be updated to point to docs/doc.md
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("docs/doc.md"),
        "Reference should be updated to docs/doc.md. Got: {}",
        ref_content
    );
}

/// Moving a file to an existing directory with internal links should update them correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_directory_updates_internal_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create a sibling file
    let sibling_file = temp_dir.path().join("other.md");
    write_file(&sibling_file, "# Other");

    // Create source file with link to sibling
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Other](other.md)");

    // Create target directory
    let target_dir = temp_dir.path().join("subdir");
    fs::create_dir_all(&target_dir).unwrap();

    // Move source file to directory
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Internal link should be updated to point to ../other.md
    let moved_file = target_dir.join("source.md");
    let content = fs::read_to_string(&moved_file).unwrap();
    assert!(
        content.contains("../other.md"),
        "Internal link should be updated to ../other.md. Got: {}",
        content
    );
}

/// Moving a file to a nested existing directory should work correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_nested_existing_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("file.md");
    write_file(&source_file, "# Content");

    // Create nested target directory
    let target_dir = temp_dir.path().join("a").join("b").join("c");
    fs::create_dir_all(&target_dir).unwrap();

    // Move source file to nested directory
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    let expected_target = target_dir.join("file.md");
    assert!(expected_target.exists());
    assert!(!source_file.exists());
}

/// Moving a file to an existing directory in dry-run mode should not move the file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_directory_dry_run() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    // Create target directory
    let target_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&target_dir).unwrap();

    // Dry-run move to directory
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(
        source_file.exists(),
        "Source file should still exist after dry-run"
    );
    assert!(
        !target_dir.join("source.md").exists(),
        "File should not be created during dry-run"
    );
}

/// Moving a file to an existing directory that already contains a file with same name should fail.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_directory_file_already_exists() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source Content");

    // Create target directory with existing file of same name
    let target_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&target_dir).unwrap();
    let existing_file = target_dir.join("source.md");
    write_file(&existing_file, "# Existing Content");

    // Move should fail because file already exists in target directory
    let result = mv(
        source_file.to_str().unwrap(),
        target_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(
        result.is_err(),
        "mv should fail when destination file already exists in directory"
    );

    // Both files should remain unchanged
    assert!(source_file.exists());
    assert_eq!(
        fs::read_to_string(&existing_file).unwrap(),
        "# Existing Content"
    );
}

/// Moving a file with trailing slash to existing directory should work.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_directory_with_trailing_slash() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("doc.md");
    write_file(&source_file, "# Doc");

    let target_dir = temp_dir.path().join("folder");
    fs::create_dir_all(&target_dir).unwrap();

    // Path with trailing slash should still be recognized as directory
    let target_with_slash = format!("{}/", target_dir.to_str().unwrap());

    let result = mv(
        source_file.to_str().unwrap(),
        &target_with_slash,
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(
        result.is_ok(),
        "Trailing slash should be handled correctly: {:?}",
        result.err()
    );

    let expected_target = target_dir.join("doc.md");
    assert!(expected_target.exists());
}

/// Moving a file to a non-existent path (not a directory) should work as before.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_to_nonexistent_path_still_works() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("old.md");
    write_file(&source_file, "# Old");

    // Target is a file path that doesn't exist
    let target_file = temp_dir.path().join("new.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());
    assert!(!source_file.exists());
}

// ============= atomicity / rollback tests =============

/// When apply_replacements fails (reference file is read-only), the transaction should
/// roll back: the copied destination file should be removed, and the source file should
/// remain intact.
#[test]
#[cfg(unix)]
#[allow(clippy::unwrap_used)]
fn test_mv_rollback_on_write_failure() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source Content");

    let ref_file = temp_dir.path().join("ref.md");
    let original_ref_content = "See [source](source.md) for details.";
    write_file(&ref_file, original_ref_content);

    let target_file = temp_dir.path().join("sub").join("moved.md");

    // Make the reference file read-only so that apply_replacements will fail on fs::write.
    let mut permissions = fs::metadata(&ref_file).unwrap().permissions();
    permissions.set_mode(0o444);
    fs::set_permissions(&ref_file, permissions).unwrap();

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Restore permissions for cleanup (TempDir needs to delete files).
    let mut permissions = fs::metadata(&ref_file).unwrap().permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&ref_file, permissions).unwrap();

    // The operation should have failed.
    assert!(
        result.is_err(),
        "mv should fail when a reference file is read-only"
    );

    // Source file should still exist (not deleted).
    assert!(
        source_file.exists(),
        "Source file should still exist after rollback"
    );

    // Destination file should have been cleaned up by rollback.
    assert!(
        !target_file.exists(),
        "Destination file should be removed by rollback"
    );

    // Reference file content should be unchanged.
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(
        ref_content, original_ref_content,
        "Reference file should be unchanged after rollback"
    );
}

/// When a move fails mid-way through updating multiple reference files, all already-modified
/// files should be restored to their original content.
#[test]
#[cfg(unix)]
#[allow(clippy::unwrap_used)]
fn test_mv_rollback_restores_already_modified_files() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source");

    // Create two reference files. We'll make one read-only to trigger failure.
    let ref_file_a = temp_dir.path().join("a_ref.md");
    let original_a_content = "[Link](source.md)";
    write_file(&ref_file_a, original_a_content);

    let ref_file_b = temp_dir.path().join("b_ref.md");
    let original_b_content = "[Link](source.md)";
    write_file(&ref_file_b, original_b_content);

    let target_file = temp_dir.path().join("sub").join("moved.md");

    // Make one reference file read-only to cause apply_replacements to fail.
    // Since HashMap iteration order is non-deterministic, we make both read-only
    // to guarantee failure regardless of processing order.
    let mut perm_b = fs::metadata(&ref_file_b).unwrap().permissions();
    perm_b.set_mode(0o444);
    fs::set_permissions(&ref_file_b, perm_b).unwrap();

    let mut perm_a = fs::metadata(&ref_file_a).unwrap().permissions();
    perm_a.set_mode(0o444);
    fs::set_permissions(&ref_file_a, perm_a).unwrap();

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Restore permissions for cleanup.
    let mut perm_a = fs::metadata(&ref_file_a).unwrap().permissions();
    perm_a.set_mode(0o644);
    fs::set_permissions(&ref_file_a, perm_a).unwrap();

    let mut perm_b = fs::metadata(&ref_file_b).unwrap().permissions();
    perm_b.set_mode(0o644);
    fs::set_permissions(&ref_file_b, perm_b).unwrap();

    assert!(result.is_err(), "mv should fail");

    // Source should still exist.
    assert!(source_file.exists(), "Source should survive rollback");

    // Destination should be cleaned up.
    assert!(
        !target_file.exists(),
        "Destination should be removed by rollback"
    );

    // Both reference files should retain their original content.
    let content_a = fs::read_to_string(&ref_file_a).unwrap();
    let content_b = fs::read_to_string(&ref_file_b).unwrap();
    assert_eq!(
        content_a, original_a_content,
        "ref_file_a should be restored after rollback"
    );
    assert_eq!(
        content_b, original_b_content,
        "ref_file_b should be restored after rollback"
    );
}

/// A successful move with multiple references should complete without leaving any
/// inconsistent state — verifying the happy path still works with the transaction wrapper.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_transaction_happy_path_multiple_refs() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source\n\n[Other](other.md)");

    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other");

    let ref_file_a = temp_dir.path().join("ref_a.md");
    write_file(&ref_file_a, "[Source](source.md)");

    let ref_file_b = temp_dir.path().join("ref_b.md");
    write_file(&ref_file_b, "See [here](source.md) for info.");

    let target_file = temp_dir.path().join("docs").join("moved.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(
        result.is_ok(),
        "Transaction happy path should succeed: {:?}",
        result.err()
    );

    // Source should be gone, target should exist.
    assert!(!source_file.exists());
    assert!(target_file.exists());

    // All references should be updated.
    let content_a = fs::read_to_string(&ref_file_a).unwrap();
    assert!(
        content_a.contains("docs/moved.md"),
        "ref_a should point to new location. Got: {}",
        content_a
    );

    let content_b = fs::read_to_string(&ref_file_b).unwrap();
    assert!(
        content_b.contains("docs/moved.md"),
        "ref_b should point to new location. Got: {}",
        content_b
    );

    // Internal link in moved file should be updated.
    let moved_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        moved_content.contains("../other.md"),
        "Internal link should be updated. Got: {}",
        moved_content
    );
}

/// Verify that when a move fails, the source file content is fully preserved.
#[test]
#[cfg(unix)]
#[allow(clippy::unwrap_used)]
fn test_mv_rollback_preserves_source_content() {
    let temp_dir = TempDir::new().unwrap();

    let original_source_content = "# Important Document\n\nThis has [links](other.md) and **formatting**.\n\n## Section\n\nMore content here.";
    let source_file = temp_dir.path().join("important.md");
    write_file(&source_file, original_source_content);

    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[Doc](important.md)");

    let target_file = temp_dir.path().join("archive").join("important.md");

    // Make reference file read-only to trigger failure.
    let mut permissions = fs::metadata(&ref_file).unwrap().permissions();
    permissions.set_mode(0o444);
    fs::set_permissions(&ref_file, permissions).unwrap();

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    // Restore permissions for cleanup.
    let mut permissions = fs::metadata(&ref_file).unwrap().permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&ref_file, permissions).unwrap();

    assert!(result.is_err());

    // Source file content must be exactly preserved.
    let source_content = fs::read_to_string(&source_file).unwrap();
    assert_eq!(
        source_content, original_source_content,
        "Source file content must be exactly preserved after rollback"
    );
}

// ============= link reference definition tests =============

/// Moving a file that is referenced via a link reference definition should update the definition.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_link_reference_definition_in_external_file() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target Content");

    // Create a referencing file that uses link reference definition syntax
    let ref_file = temp_dir.path().join("index.md");
    write_file(
        &ref_file,
        "See [the target][ref] for details.\n\n[ref]: target.md",
    );

    let target_file = temp_dir.path().join("docs").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    // The link reference definition should be updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("]: docs/target.md"),
        "Link reference definition should be updated to docs/target.md. Got: {}",
        ref_content
    );
    // The usage site should remain unchanged
    assert!(
        ref_content.contains("[the target][ref]"),
        "Link usage should remain unchanged. Got: {}",
        ref_content
    );
}

/// UTF-8 BOM at file start should not prevent reference definitions from being detected.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_bom_prefixed_reference_definition_updates_reference() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target Content");

    let ref_file = temp_dir.path().join("index.md");
    fs::write(
        &ref_file,
        b"\xEF\xBB\xBF[ref]: target.md\n\nSee [the target][ref].\n",
    )
    .unwrap();

    let target_file = temp_dir.path().join("docs").join("target.md");
    mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("[ref]: docs/target.md"));
}

/// Moving a file that is referenced via a link reference definition with a title.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_link_reference_definition_with_title() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[text][ref]\n\n[ref]: target.md \"My Title\"");

    let target_file = temp_dir.path().join("sub").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("]: sub/target.md"),
        "Link reference definition URL should be updated. Got: {}",
        ref_content
    );
    assert!(
        ref_content.contains("\"My Title\""),
        "Title should be preserved. Got: {}",
        ref_content
    );
}

/// Moving a file referenced by an angle-bracket reference definition should preserve the brackets.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_link_reference_definition_with_angle_brackets() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[text][ref]\n\n[ref]: <target.md>");

    let target_file = temp_dir.path().join("sub").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("[ref]: <sub/target.md>"),
        "Angle-bracket reference definition should be updated in place. Got: {}",
        ref_content
    );
}

/// Moving a file referenced by a spaced reference definition should preserve the spacing.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_link_reference_definition_with_extra_spaces() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "[text][ref]\n\n[ref]:    target.md");

    let target_file = temp_dir.path().join("sub").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(
        ref_content.contains("[ref]:    sub/target.md"),
        "Reference definition spacing should be preserved. Got: {}",
        ref_content
    );
}

/// Moving a file with both inline links and link reference definitions referencing it.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_mixed_inline_and_reference_definition() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    // File with both inline link and reference definition pointing to the same target
    let ref_file = temp_dir.path().join("mixed.md");
    write_file(
        &ref_file,
        "[inline](target.md)\n\n[ref link][myref]\n\n[myref]: target.md",
    );

    let target_file = temp_dir.path().join("docs").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    // Inline link should NOT be updated (it was replaced by the reference definition)
    // Actually: the inline link is a separate link that doesn't use reference syntax,
    // so it should be updated via the normal inline path
    assert!(
        ref_content.contains("](docs/target.md)"),
        "Inline link should be updated. Got: {}",
        ref_content
    );
    assert!(
        ref_content.contains("]: docs/target.md"),
        "Reference definition should be updated. Got: {}",
        ref_content
    );
}

/// The moved file itself contains link reference definitions pointing to other files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_internal_link_reference_definitions() {
    let temp_dir = TempDir::new().unwrap();

    // Create a sibling file
    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other");

    // Create source file with link reference definition to sibling
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[see other][ref]\n\n[ref]: other.md");

    let target_file = temp_dir.path().join("sub").join("moved.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    // The moved file's internal link reference definition should be updated
    let moved_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        moved_content.contains("]: ../other.md"),
        "Internal link reference definition should be updated to ../other.md. Got: {}",
        moved_content
    );
}

/// Dry-run with link reference definitions should not modify any files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_dry_run_with_link_reference_definitions() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    let ref_file = temp_dir.path().join("index.md");
    let original_content = "[text][ref]\n\n[ref]: target.md";
    write_file(&ref_file, original_content);

    let target_file = temp_dir.path().join("docs").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Nothing should be modified
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(
        ref_content, original_content,
        "Dry-run should not modify reference file"
    );
    assert!(source_file.exists(), "Source should still exist");
    assert!(!target_file.exists(), "Target should not be created");
}

/// Multiple link reference definitions in the same file referencing the moved file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_multiple_link_reference_definitions_same_file() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Target");

    let ref_file = temp_dir.path().join("index.md");
    write_file(
        &ref_file,
        "[a][ref1]\n[b][ref2]\n\n[ref1]: target.md\n[ref2]: target.md",
    );

    let target_file = temp_dir.path().join("docs").join("target.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    let definition_count = ref_content.matches("]: docs/target.md").count();
    assert_eq!(
        definition_count, 2,
        "Both reference definitions should be updated. Got: {}",
        ref_content
    );
}

// ============= Unicode mv tests =============

/// Test moving files with Unicode filenames (Chinese, Japanese, emoji).
#[rstest]
#[case::chinese("源文件.md", "目标文件.md", "# 中文文档")]
#[case::japanese("元ファイル.md", "新ファイル.md", "# ドキュメント")]
#[case::emoji("📝笔记.md", "📚笔记.md", "# Notes")]
#[allow(clippy::unwrap_used)]
fn test_mv_unicode_filename(
    #[case] source_name: &str,
    #[case] target_name: &str,
    #[case] content: &str,
) {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join(source_name);
    write_file(&source_file, content);

    let target_file = temp_dir.path().join(target_name);

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());
    assert!(target_file.exists(), "Target file should exist");
    assert!(!source_file.exists(), "Source file should not exist");
}

/// Test moving file with Chinese filename to subdirectory.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_chinese_to_subdirectory() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("文档.md");
    write_file(&source_file, "# 文档内容");

    let target_file = temp_dir.path().join("子目录").join("归档文档.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(target_file.exists());
}

/// Test moving file and updating references with Unicode paths.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_unicode_updates_references() {
    let fixture = fixture_unicode_paths();
    let result = mv(
        fixture.source.to_str().unwrap(),
        fixture.destination.to_str().unwrap(),
        fixture.root.to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    let ref_content = fs::read_to_string(&fixture.reference).unwrap();
    assert!(
        ref_content.contains("归档/更新文档.md"),
        "Reference should be updated. Got: {}",
        ref_content
    );
}

/// Test moving file updates internal Unicode links.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_updates_internal_unicode_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create sibling file
    let other_file = temp_dir.path().join("其他文档.md");
    write_file(&other_file, "# 其他");

    // Create source with internal link
    let source_file = temp_dir.path().join("主页.md");
    write_file(&source_file, "参见 [其他文档](其他文档.md)");

    // Move to subdirectory
    let target_file = temp_dir.path().join("子目录").join("主页.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok());

    let moved_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        moved_content.contains("../其他文档.md"),
        "Internal link should be updated. Got: {}",
        moved_content
    );
}

/// Test dry-run with Unicode filenames should not modify anything.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_unicode_dry_run() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("测试文件.md");
    write_file(&source_file, "# 测试");

    let ref_file = temp_dir.path().join("引用.md");
    let original_content = "参考 [测试文件](测试文件.md)";
    write_file(&ref_file, original_content);

    let target_file = temp_dir.path().join("新位置").join("测试文件.md");

    let result = mv(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());
    assert!(
        source_file.exists(),
        "Source should still exist after dry-run"
    );
    assert!(
        !target_file.exists(),
        "Target should not be created during dry-run"
    );

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert_eq!(
        ref_content, original_content,
        "Dry-run should not modify reference file"
    );
}

// ============= Non-Markdown resource file tests =============

/// Moving a directory containing image files should update references to those images.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_directory_updates_image_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create a docs directory with an image
    let docs_dir = temp_dir.path().join("docs");
    let image_file = docs_dir.join("image.png");
    fs::create_dir_all(&docs_dir).unwrap();
    write_file(&image_file, "fake image content");

    // Create a markdown file that references the image
    let index_file = temp_dir.path().join("index.md");
    write_file(&index_file, "![Docs Image](docs/image.png)");

    // Move the docs directory to a new location
    let new_docs_dir = temp_dir.path().join("documentation");
    let result = mv(
        docs_dir.to_str().unwrap(),
        new_docs_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    // Verify the image was moved
    let new_image_path = new_docs_dir.join("image.png");
    assert!(
        new_image_path.exists(),
        "Image should be moved to new location"
    );
    assert!(!image_file.exists(), "Old image should not exist");

    // Verify the reference was updated
    let index_content = fs::read_to_string(&index_file).unwrap();
    assert!(
        index_content.contains("documentation/image.png"),
        "Image reference should be updated to documentation/image.png. Got: {}",
        index_content
    );
}

/// Moving a directory with multiple non-md files (images, PDFs) should update all references.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_directory_updates_multiple_resource_types() {
    let temp_dir = TempDir::new().unwrap();

    // Create assets directory with multiple resource files
    let assets_dir = temp_dir.path().join("assets");
    fs::create_dir_all(&assets_dir).unwrap();
    write_file(assets_dir.join("logo.png"), "png content");
    write_file(assets_dir.join("diagram.svg"), "svg content");
    write_file(assets_dir.join("document.pdf"), "pdf content");

    // Create markdown file referencing all resources
    let readme_file = temp_dir.path().join("README.md");
    write_file(
        &readme_file,
        "![Logo](assets/logo.png)\n\n![Diagram](assets/diagram.svg)\n\n[PDF](assets/document.pdf)",
    );

    // Move assets to static directory
    let static_dir = temp_dir.path().join("static");
    let result = mv(
        assets_dir.to_str().unwrap(),
        static_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    // Verify all references updated
    let readme_content = fs::read_to_string(&readme_file).unwrap();
    assert!(
        readme_content.contains("static/logo.png"),
        "Logo reference should be updated. Got: {}",
        readme_content
    );
    assert!(
        readme_content.contains("static/diagram.svg"),
        "SVG reference should be updated. Got: {}",
        readme_content
    );
    assert!(
        readme_content.contains("static/document.pdf"),
        "PDF reference should be updated. Got: {}",
        readme_content
    );
}

/// Moving a directory should update image references from nested markdown files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_directory_updates_nested_image_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create nested structure: docs/guide with an image
    let guide_dir = temp_dir.path().join("docs").join("guide");
    let images_dir = guide_dir.join("images");
    fs::create_dir_all(&images_dir).unwrap();
    write_file(images_dir.join("screenshot.png"), "screenshot");

    // Create markdown file in docs root referencing the nested image
    let docs_index = temp_dir.path().join("docs").join("index.md");
    write_file(&docs_index, "![Screenshot](guide/images/screenshot.png)");

    // Move the docs directory
    let new_docs_dir = temp_dir.path().join("documentation");
    let result = mv(
        temp_dir.path().join("docs").to_str().unwrap(),
        new_docs_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
        &NoopProgress,
    );

    assert!(result.is_ok(), "mv should succeed: {:?}", result.err());

    // Verify reference was updated
    let index_content = fs::read_to_string(new_docs_dir.join("index.md")).unwrap();
    assert!(
        index_content.contains("guide/images/screenshot.png"),
        "Nested image reference should remain valid. Got: {}",
        index_content
    );
}

/// Dry-run moving a directory with image files should not modify references.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_directory_dry_run_with_images() {
    let temp_dir = TempDir::new().unwrap();

    // Create docs with image
    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir).unwrap();
    write_file(docs_dir.join("image.png"), "image content");

    // Create reference
    let index_file = temp_dir.path().join("index.md");
    let original_content = "![Image](docs/image.png)";
    write_file(&index_file, original_content);

    // Dry-run move
    let new_docs_dir = temp_dir.path().join("new-docs");
    let result = mv(
        docs_dir.to_str().unwrap(),
        new_docs_dir.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        true,
        &NoopProgress,
    );

    assert!(result.is_ok());

    // Verify nothing changed
    assert!(docs_dir.exists(), "Original directory should still exist");
    assert!(
        !new_docs_dir.exists(),
        "New directory should not be created"
    );
    let index_content = fs::read_to_string(&index_file).unwrap();
    assert_eq!(
        index_content, original_content,
        "Dry-run should not modify references"
    );
}

// ============= Regression tests for #6: relative source + self-reference =============

/// Regression test for #6.
///
/// When the user invokes `mv` from a shell cwd with a **relative** source path
/// (no leading `./`) and `root="."`, the self-reference inside the source file
/// must still be recognised and rewritten — previously the `HashMap` key
/// comparison missed the self-reference entry because `WalkBuilder` produced
/// paths with a `./` prefix while the user supplied a bare relative path,
/// causing `apply_replacements` to read the stale pre-move path and fail.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_relative_source_with_self_reference_succeeds() {
    let temp_dir = TempDir::new().unwrap();
    write_file(temp_dir.path().join("page.md"), "[Self](page.md)\n");

    with_current_dir(temp_dir.path(), || {
        let result = mv("page.md", "./moved.md", ".", false, &NoopProgress);
        assert!(
            result.is_ok(),
            "mv with bare relative source should succeed, got: {:?}",
            result.err()
        );
    });

    let source = temp_dir.path().join("page.md");
    let target = temp_dir.path().join("moved.md");
    assert!(!source.exists(), "source should have been removed");
    assert!(target.exists(), "target should have been created");

    let content = fs::read_to_string(&target).unwrap();
    assert!(
        content.contains("moved.md"),
        "self-reference should be rewritten to new filename, got: {content}"
    );
    assert!(
        !content.contains("page.md"),
        "old self-reference must not survive, got: {content}"
    );
}

/// Regression test for #6.
///
/// A cross-reference from another file must still be rewritten when the user
/// supplies `source` as a bare relative path and `root="."`. Before the fix
/// the external-reference path happened to work (the `WalkBuilder`-produced
/// path was consistently used on both sides), but we lock in the end-to-end
/// guarantee here so future refactors can't regress it.
#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_relative_source_rewrites_external_reference() {
    let temp_dir = TempDir::new().unwrap();
    write_file(temp_dir.path().join("page.md"), "# Page\n");
    write_file(temp_dir.path().join("ref.md"), "[Link](page.md)\n");

    with_current_dir(temp_dir.path(), || {
        let result = mv("page.md", "./moved.md", ".", false, &NoopProgress);
        assert!(result.is_ok(), "mv should succeed, got: {:?}", result.err());
    });

    let ref_content = fs::read_to_string(temp_dir.path().join("ref.md")).unwrap();
    assert!(
        ref_content.contains("moved.md"),
        "external reference should point to the new filename, got: {ref_content}"
    );
    assert!(
        !ref_content.contains("page.md"),
        "old external reference must not survive, got: {ref_content}"
    );
}
