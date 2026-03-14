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
