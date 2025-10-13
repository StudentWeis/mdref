use mdref::{find_links, find_references, Reference};
use std::path::Path;

// ============= find_links tests =============

#[test]
fn test_find_links_nonexistent_file() {
    let result = find_links(Path::new("nonexistent.md"));
    assert!(result.is_err());
}

#[test]
fn test_find_links_basic() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_find_links_non_markdown_file() {
    // Non-markdown files should return an empty Vec
    let path = Path::new("Cargo.toml");
    let result = find_links(path).unwrap();
    assert!(result.is_empty());
}

#[test]
fn test_find_links_count() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();
    // main.md should have 8 links (including image links)
    assert_eq!(result.len(), 8);
}

#[test]
fn test_find_links_content_verification() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();
    
    // Verify the found links
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    assert!(link_texts.contains(&"main.md"));
    assert!(link_texts.contains(&"inner/main.md"));
    assert!(link_texts.contains(&"other.md"));
}

#[test]
fn test_find_links_line_numbers() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();
    
    // Verify that line numbers are correct (greater than 0)
    for reference in result {
        assert!(reference.line > 0);
        assert!(reference.column > 0);
    }
}

// ============= find_references tests =============

#[test]
fn test_find_references_basic() {
    let path = Path::new("examples/main.md");
    let result = find_references(path, path.parent().unwrap()).unwrap();
    assert_eq!(result.len(), 6)
}

#[test]
fn test_find_references_nonexistent_file() {
    let path = Path::new("nonexistent.md");
    let root = Path::new("examples");
    let result = find_references(path, root);
    assert!(result.is_err());
}

#[test]
fn test_find_references_other_md() {
    let path = Path::new("examples/other.md");
    let result = find_references(path, path.parent().unwrap()).unwrap();
    // other.md is referenced once by main.md
    assert!(!result.is_empty());
}

#[test]
fn test_find_references_inner_main() {
    let path = Path::new("examples/inner/main.md");
    let root = Path::new("examples");
    let result = find_references(path, root).unwrap();
    // inner/main.md is referenced by the outer main.md and other.md
    assert!(result.len() >= 2);
}

#[test]
fn test_find_references_empty_directory() {
    let path = Path::new("examples/main.md");
    // Use a directory that should have no references
    let root = Path::new("benches");
    let result = find_references(path, root).unwrap();
    // The benches directory should have no references to examples/main.md
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_references_returns_correct_paths() {
    let path = Path::new("examples/main.md");
    let result = find_references(path, path.parent().unwrap()).unwrap();
    
    // Verify that the returned paths are all markdown files
    for reference in result {
        assert_eq!(reference.path.extension().and_then(|s| s.to_str()), Some("md"));
    }
}

// ============= Reference struct tests =============

#[test]
fn test_reference_creation() {
    let reference = Reference::new(
        std::path::PathBuf::from("test.md"),
        10,
        5,
        "link.md".to_string()
    );
    
    assert_eq!(reference.line, 10);
    assert_eq!(reference.column, 5);
    assert_eq!(reference.link_text, "link.md");
}

#[test]
fn test_reference_display() {
    let reference = Reference::new(
        std::path::PathBuf::from("test.md"),
        10,
        5,
        "link.md".to_string()
    );
    
    let display_str = format!("{}", reference);
    assert!(display_str.contains("test.md"));
    assert!(display_str.contains("10"));
    assert!(display_str.contains("5"));
    assert!(display_str.contains("link.md"));
}

// ============= Edge case tests =============

#[test]
fn test_find_links_empty_markdown_file() {
    // Create a temporary empty file for testing
    use std::fs;
    use std::io::Write;
    
    let temp_file = "test_empty.md";
    fs::File::create(temp_file).unwrap().write_all(b"").unwrap();
    
    let result = find_links(Path::new(temp_file)).unwrap();
    assert_eq!(result.len(), 0);
    
    // Cleanup
    fs::remove_file(temp_file).ok();
}

#[test]
fn test_find_references_with_relative_paths() {
    let path = Path::new("examples/main.md");
    let result = find_references(path, "examples").unwrap();
    assert!(!result.is_empty());
}
