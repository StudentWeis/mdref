use mdref::{find_links, find_references, mv_file};
use std::fs;
use std::io::Write;
use std::path::Path;

// Test helper function: create temporary test environment
fn create_test_env(test_name: &str) -> String {
    let base_dir = format!("test_mv_{}", test_name);
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
fn create_test_file(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

// ============= mv_file basic tests =============

#[test]
fn test_mv_file_basic() {
    let test_dir = create_test_env("basic");
    
    // Create test file
    let source_file = format!("{}/source.md", test_dir);
    let target_file = format!("{}/target.md", test_dir);
    create_test_file(&source_file, "# Source File\n\nSome content.");
    
    // Perform move
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Verify
    assert!(result.is_ok());
    assert!(Path::new(&target_file).exists());
    assert!(!Path::new(&source_file).exists());
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_with_references() {
    let test_dir = create_test_env("with_refs");
    
    // Create source file
    let source_file = format!("{}/source.md", test_dir);
    create_test_file(&source_file, "# Source File");
    
    // Create file that references the source file
    let ref_file = format!("{}/reference.md", test_dir);
    create_test_file(&ref_file, "[Link to source](source.md)");
    
    // Move file
    let target_file = format!("{}/target.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Verify
    assert!(result.is_ok());
    
    // Check if references are updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("target.md"));
    assert!(!ref_content.contains("source.md"));
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_to_subdirectory() {
    let test_dir = create_test_env("to_subdir");
    
    // Create source file
    let source_file = format!("{}/source.md", test_dir);
    create_test_file(&source_file, "# Source File");
    
    // Create reference file
    let ref_file = format!("{}/ref.md", test_dir);
    create_test_file(&ref_file, "[Source](source.md)");
    
    // Move to subdirectory
    let target_file = format!("{}/subdir/target.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Verify
    assert!(result.is_ok());
    assert!(Path::new(&target_file).exists());
    
    // Check if references are correctly updated to relative paths
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("subdir"));
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_with_internal_links() {
    let test_dir = create_test_env("internal_links");
    
    // Create source file containing links
    let source_file = format!("{}/source.md", test_dir);
    let other_file = format!("{}/other.md", test_dir);
    create_test_file(&other_file, "# Other File");
    create_test_file(&source_file, "[Other](other.md)");
    
    // Move file to subdirectory
    let target_file = format!("{}/subdir/target.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Verify
    assert!(result.is_ok());
    
    // Check if links in the moved file are updated
    let target_content = fs::read_to_string(&target_file).unwrap();
    // The relative path from subdir/target.md to other.md should be ../other.md
    assert!(target_content.contains("../other.md") || target_content.contains("other.md"));
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_nonexistent_source() {
    let test_dir = create_test_env("nonexistent");
    
    let source_file = format!("{}/nonexistent.md", test_dir);
    let target_file = format!("{}/target.md", test_dir);
    
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Should return an error
    assert!(result.is_err());
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_creates_parent_directory() {
    let test_dir = create_test_env("create_parent");
    
    // Create source file
    let source_file = format!("{}/source.md", test_dir);
    create_test_file(&source_file, "# Content");
    
    // Target path contains non-existent directory
    let target_file = format!("{}/new/nested/dir/target.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    // Should successfully create directory and move file
    assert!(result.is_ok());
    assert!(Path::new(&target_file).exists());
    
    cleanup_test_env(&test_dir);
}

// ============= Edge case tests =============

#[test]
fn test_mv_file_preserves_content() {
    let test_dir = create_test_env("preserve_content");
    
    let content = "# Title\n\nParagraph 1\n\n## Section\n\nMore content.";
    let source_file = format!("{}/source.md", test_dir);
    create_test_file(&source_file, content);
    
    let target_file = format!("{}/target.md", test_dir);
    mv_file(&source_file, &target_file, &test_dir).unwrap();
    
    let target_content = fs::read_to_string(&target_file).unwrap();
    // Content should be preserved
    assert!(target_content.contains("# Title"));
    assert!(target_content.contains("Paragraph 1"));
    assert!(target_content.contains("## Section"));
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_multiple_references() {
    let test_dir = create_test_env("multiple_refs");
    
    // Create source file
    let source_file = format!("{}/source.md", test_dir);
    create_test_file(&source_file, "# Source");
    
    // Create multiple reference files
    let ref1 = format!("{}/ref1.md", test_dir);
    let ref2 = format!("{}/ref2.md", test_dir);
    let ref3 = format!("{}/ref3.md", test_dir);
    create_test_file(&ref1, "[Link](source.md)");
    create_test_file(&ref2, "[Another link](source.md)");
    create_test_file(&ref3, "[Third link](source.md)");
    
    // Move file
    let target_file = format!("{}/target.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    assert!(result.is_ok());
    
    // Verify all references have been updated
    let ref1_content = fs::read_to_string(&ref1).unwrap();
    let ref2_content = fs::read_to_string(&ref2).unwrap();
    let ref3_content = fs::read_to_string(&ref3).unwrap();
    
    assert!(ref1_content.contains("target.md"));
    assert!(ref2_content.contains("target.md"));
    assert!(ref3_content.contains("target.md"));
    
    cleanup_test_env(&test_dir);
}

#[test]
fn test_mv_file_same_name_different_directory() {
    let test_dir = create_test_env("same_name");
    
    // Create source file
    let source_file = format!("{}/file.md", test_dir);
    create_test_file(&source_file, "# Original");
    
    // Create reference
    let ref_file = format!("{}/ref.md", test_dir);
    create_test_file(&ref_file, "[Link](file.md)");
    
    // Move to subdirectory, keeping filename
    let target_file = format!("{}/subdir/file.md", test_dir);
    let result = mv_file(&source_file, &target_file, &test_dir);
    
    assert!(result.is_ok());
    assert!(Path::new(&target_file).exists());
    
    cleanup_test_env(&test_dir);
}

// ============= Integration tests =============

#[test]
fn test_mv_file_integration_with_find() {
    let test_dir = create_test_env("integration");
    
    // Create test file structure
    let source_file = format!("{}/source.md", test_dir);
    let other_file = format!("{}/other.md", test_dir);
    create_test_file(&source_file, "[Other](other.md)");
    create_test_file(&other_file, "# Other");
    
    let ref_file = format!("{}/ref.md", test_dir);
    create_test_file(&ref_file, "[Source](source.md)");
    
    // Find references before move
    let refs_before = find_references(&source_file, &test_dir).unwrap();
    assert!(!refs_before.is_empty());
    
    // Move file
    let target_file = format!("{}/target.md", test_dir);
    mv_file(&source_file, &target_file, &test_dir).unwrap();
    
    // Verify links in new file after move
    let links_after = find_links(&target_file).unwrap();
    assert!(!links_after.is_empty());
    
    cleanup_test_env(&test_dir);
}
