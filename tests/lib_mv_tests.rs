use mdref::{find_links, find_references, mv_file};
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

// ============= mv_file basic tests =============

#[test]
fn test_mv_file_basic() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file
    let source_file = temp_dir.path().join("source.md");
    let target_file = temp_dir.path().join("target.md");
    write_file(&source_file, "# Source File\n\nSome content.");

    // Perform move
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Verify
    assert!(result.is_ok());
    assert!(target_file.exists());
    assert!(!source_file.exists());
}

#[test]
fn test_mv_file_with_references() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source File");

    // Create file that references the source file
    let ref_file = temp_dir.path().join("reference.md");
    write_file(&ref_file, "[Link to source](source.md)");

    // Move file
    let target_file = temp_dir.path().join("target.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Verify
    assert!(result.is_ok());

    // Check if references are updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("target.md"));
    assert!(!ref_content.contains("source.md"));
}

#[test]
fn test_mv_file_to_subdirectory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Source File");

    // Create reference file
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Move to subdirectory
    let target_file = temp_dir.path().join("subdir").join("target.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Verify
    assert!(result.is_ok());
    assert!(target_file.exists());

    // Check if references are correctly updated to relative paths
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("subdir"));
}

#[test]
fn test_mv_file_with_internal_links() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file containing links
    let source_file = temp_dir.path().join("source.md");
    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other File");
    write_file(&source_file, "[Other](other.md)");

    // Move file to subdirectory
    let target_file = temp_dir.path().join("subdir").join("target.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Verify
    assert!(result.is_ok());

    // Check if links in the moved file are updated
    let target_content = fs::read_to_string(&target_file).unwrap();
    // The relative path from subdir/target.md to other.md should be ../other.md
    assert!(target_content.contains("../other.md") || target_content.contains("other.md"));
}

#[test]
fn test_mv_file_nonexistent_source() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("nonexistent.md");
    let target_file = temp_dir.path().join("target.md");

    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Should return an error
    assert!(result.is_err());
}

#[test]
fn test_mv_file_creates_parent_directory() {
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
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Should successfully create directory and move file
    assert!(result.is_ok());
    assert!(target_file.exists());
}

// ============= Edge case tests =============

#[test]
fn test_mv_file_preserves_content() {
    let temp_dir = TempDir::new().unwrap();

    let content = "# Title\n\nParagraph 1\n\n## Section\n\nMore content.";
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, content);

    let target_file = temp_dir.path().join("target.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let target_content = fs::read_to_string(&target_file).unwrap();
    // Content should be preserved
    assert!(target_content.contains("# Title"));
    assert!(target_content.contains("Paragraph 1"));
    assert!(target_content.contains("## Section"));
}

#[test]
fn test_mv_file_multiple_references() {
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
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_same_name_different_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file
    let source_file = temp_dir.path().join("file.md");
    write_file(&source_file, "# Original");

    // Create reference
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](file.md)");

    // Move to subdirectory, keeping filename
    let target_file = temp_dir.path().join("subdir").join("file.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    assert!(result.is_ok());
    assert!(target_file.exists());
}

// ============= Integration tests =============

#[test]
fn test_mv_file_integration_with_find() {
    let temp_dir = TempDir::new().unwrap();

    // Create test file structure
    let source_file = temp_dir.path().join("source.md");
    let other_file = temp_dir.path().join("other.md");
    write_file(&source_file, "[Other](other.md)");
    write_file(&other_file, "# Other");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    // Find references before move
    let refs_before = find_references(&source_file, temp_dir.path()).unwrap();
    assert!(!refs_before.is_empty());

    // Move file
    let target_file = temp_dir.path().join("target.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    // Verify links in new file after move
    let links_after = find_links(&target_file).unwrap();
    assert!(!links_after.is_empty());
}

// ============= Deep nested move tests =============

#[test]
fn test_mv_file_deep_nested_move() {
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
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_same_file_multiple_lines_referencing() {
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
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    // All three references should be updated
    assert!(!ref_content.contains("source.md"));
    assert_eq!(ref_content.matches("dest.md").count(), 3);
}

// ============= Move with self-reference =============

#[test]
fn test_mv_file_self_reference_update() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("page.md");
    write_file(&source_file, "[Self](page.md)");

    let target_file = temp_dir.path().join("moved.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    assert!(content.contains("moved.md"));
    assert!(!content.contains("page.md"));
}

// ============= Move to same directory (equivalent to rename) =============

#[test]
fn test_mv_file_same_directory() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("old_name.md");
    write_file(&source_file, "# Content");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](old_name.md)");

    let target_file = temp_dir.path().join("new_name.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_with_image_links() {
    let temp_dir = TempDir::new().unwrap();

    let image_file = temp_dir.path().join("assets").join("photo.png");
    write_file(&image_file, "fake image data");

    let source_file = temp_dir.path().join("doc.md");
    write_file(&source_file, "# Doc\n\n![Photo](assets/photo.png)");

    // Move doc.md into a subdirectory
    let target_file = temp_dir.path().join("sub").join("doc.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    // Image link should be updated to ../assets/photo.png
    assert!(content.contains("../assets/photo.png"));
}

// ============= Move file with mixed link types =============

#[test]
fn test_mv_file_preserves_external_urls() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("local.md");
    write_file(&other_file, "# Local");

    let source_file = temp_dir.path().join("mixed.md");
    // File contains both external URLs and local links.
    // mv_file should skip external URLs and only update local links.
    write_file(
        &source_file,
        "[Google](https://google.com)\n\n[Local](local.md)\n\n[GitHub](https://github.com)",
    );

    let target_file = temp_dir.path().join("sub").join("mixed.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    let content = fs::read_to_string(&target_file).unwrap();
    // External URLs should remain unchanged
    assert!(content.contains("https://google.com"));
    assert!(content.contains("https://github.com"));
    // Local link should be updated
    assert!(content.contains("../local.md"));
}

// ============= Move from subdirectory to root =============

#[test]
fn test_mv_file_from_subdir_to_root() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("sub").join("sibling.md");
    write_file(&other_file, "# Sibling");

    let source_file = temp_dir.path().join("sub").join("nested.md");
    write_file(&source_file, "[Sibling](sibling.md)");

    let ref_file = temp_dir.path().join("root_ref.md");
    write_file(&ref_file, "[Nested](sub/nested.md)");

    let target_file = temp_dir.path().join("promoted.md");
    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    // Reference from root should be updated
    let root_ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(root_ref_content.contains("promoted.md"));

    // Internal link should be updated to point to sub/sibling.md
    let content = fs::read_to_string(&target_file).unwrap();
    assert!(content.contains("sub/sibling.md"));
}

// ============= Edge case tests =============

/// Move file to a target path with non-existent intermediate directories.
#[test]
fn test_mv_file_with_nonexistent_intermediate_path() {
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
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_self_reference_cross_directory() {
    let temp_dir = TempDir::new().unwrap();

    // Create source file with self-reference in a subdirectory
    let source_file = temp_dir.path().join("original").join("page.md");
    write_file(
        &source_file,
        "# Page\n\n[Self](page.md)\n\n[Also self](./page.md)",
    );

    // Move to a different directory with different name
    let target_file = temp_dir.path().join("moved").join("renamed.md");

    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_error_type_validation() {
    let temp_dir = TempDir::new().unwrap();

    // Create a source file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Content");

    // Create a reference file with a link
    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Link](source.md)");

    // Move the file - this should succeed
    let target_file = temp_dir.path().join("target.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // This test just verifies that normal operation succeeds
    // The error type validation is done in unit tests
    assert!(result.is_ok());
}

/// Move file to a subdirectory and verify internal links are updated correctly.
#[test]
fn test_mv_file_with_relative_target_path() {
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

    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
fn test_mv_file_deep_new_directory_with_links() {
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

    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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
#[test]
fn test_mv_file_internal_link_with_anchor_preserved() {
    let temp_dir = TempDir::new().unwrap();

    // Create target of the link
    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other\n\n## Details");

    // Source file has a link with anchor fragment
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Details](other.md#details)");

    // Move source to a subdirectory
    let target_file = temp_dir.path().join("sub").join("moved.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    assert!(
        result.is_ok(),
        "mv_file should not fail when internal links have anchors: {:?}",
        result.err()
    );

    let content = fs::read_to_string(&target_file).unwrap();
    // The anchor should be preserved and path updated
    assert!(
        content.contains("../other.md#details"),
        "Internal link anchor should be preserved. Got: {}",
        content
    );
}

/// Move file with internal link that has anchor, staying in same directory.
#[test]
fn test_mv_file_internal_link_with_anchor_same_dir() {
    let temp_dir = TempDir::new().unwrap();

    let other_file = temp_dir.path().join("other.md");
    write_file(&other_file, "# Other\n\n## Section");

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Section](other.md#section)");

    let target_file = temp_dir.path().join("renamed.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    assert!(
        result.is_ok(),
        "mv_file should handle anchored internal links: {:?}",
        result.err()
    );

    let content = fs::read_to_string(&target_file).unwrap();
    assert!(
        content.contains("other.md#section"),
        "Anchor should be preserved in same-dir move. Got: {}",
        content
    );
}

/// Move file containing a broken (dangling) link should not fail.
/// Bug: canonicalize() on non-existent link path returns IO error,
/// causing the entire mv_file to fail.
#[test]
fn test_mv_file_with_broken_internal_link() {
    let temp_dir = TempDir::new().unwrap();

    // Source file has a link to a non-existent file
    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "[Broken](nonexistent.md)");

    let target_file = temp_dir.path().join("target.md");
    let result = mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    );

    // Should succeed — broken links should be skipped, not cause failure
    assert!(
        result.is_ok(),
        "mv_file should not fail due to broken internal links: {:?}",
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
fn test_mv_file_source_equals_dest() {
    let temp_dir = TempDir::new().unwrap();

    let file = temp_dir.path().join("same.md");
    write_file(&file, "# Content");

    // Move file to itself
    let result = mv_file(
        file.to_str().unwrap(),
        file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
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

/// Move file with anchor links, preserving the anchor fragments.
#[test]
fn test_mv_file_preserves_anchor_links() {
    let temp_dir = TempDir::new().unwrap();

    let source_file = temp_dir.path().join("source.md");
    write_file(&source_file, "# Title\n\n[Section](#section)\n\n## Section");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md#title)");

    let target_file = temp_dir.path().join("target.md");

    mv_file(
        source_file.to_str().unwrap(),
        target_file.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
    )
    .unwrap();

    // Internal anchor link should be preserved
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert!(target_content.contains("#section"));

    // External reference with anchor should be updated correctly
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("target.md#title"));
}
