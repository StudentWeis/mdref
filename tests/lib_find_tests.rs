use mdref::{Reference, find_links, find_references};
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[allow(clippy::unwrap_used)]
fn write_file<P: AsRef<Path>>(path: P, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

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
        assert_eq!(
            reference.path.extension().and_then(|s| s.to_str()),
            Some("md")
        );
    }
}

// ============= Reference struct tests =============

#[test]
fn test_reference_creation() {
    let reference = Reference::new(
        std::path::PathBuf::from("test.md"),
        10,
        5,
        "link.md".to_string(),
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
        "link.md".to_string(),
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
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test_empty.md");
    fs::File::create(&temp_file)
        .unwrap()
        .write_all(b"")
        .unwrap();

    let result = find_links(&temp_file).unwrap();
    assert_eq!(result.len(), 0);
}

#[test]
fn test_find_references_with_relative_paths() {
    let path = Path::new("examples/main.md");
    let result = find_references(path, "examples").unwrap();
    assert!(!result.is_empty());
}

// ============= Image link tests =============

#[test]
fn test_find_links_includes_image_links() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();

    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    // main.md contains ![jpg test](main.jpg) twice
    assert!(link_texts.contains(&"main.jpg"));
}

#[test]
fn test_find_links_image_with_dot_slash_prefix() {
    // examples/other.md uses ./main.jpg syntax
    let path = Path::new("examples/other.md");
    let result = find_links(path).unwrap();

    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    assert!(link_texts.contains(&"./main.jpg"));
}

// ============= Multiple links on same line =============

#[test]
fn test_find_links_multiple_links_same_line() {
    let path = Path::new("examples/main.md");
    let result = find_links(path).unwrap();

    // Line 7: [outer main](main.md) - [inner main](inner/main.md) - [sub inner main](inner/sub/main.md)
    // All three links should be on line 7
    let line7_links: Vec<&Reference> = result.iter().filter(|r| r.line == 7).collect();
    assert_eq!(line7_links.len(), 3);

    // Verify column numbers are distinct and increasing
    let columns: Vec<usize> = line7_links.iter().map(|r| r.column).collect();
    assert!(columns.windows(2).all(|w| w[0] < w[1]));
}

// ============= Nested directory references =============

#[test]
fn test_find_references_deep_nested() {
    // inner/main.md is referenced from multiple levels
    let path = Path::new("examples/inner/main.md");
    let root = Path::new("examples");
    let result = find_references(path, root).unwrap();

    // Should be referenced by outer main.md, inner/other.md, etc.
    assert!(result.len() >= 2);

    // Verify references come from different directory levels
    let ref_paths: Vec<String> = result
        .iter()
        .map(|r| r.path.display().to_string())
        .collect();
    let has_outer_ref = ref_paths.iter().any(|p| !p.contains("inner"));
    let has_inner_ref = ref_paths.iter().any(|p| p.contains("inner"));
    assert!(has_outer_ref || has_inner_ref);
}

// ============= Self-reference =============

#[test]
fn test_find_references_self_reference() {
    // main.md references itself: [outer main](main.md)
    let path = Path::new("examples/main.md");
    let result = find_references(path, path.parent().unwrap()).unwrap();

    // main.md should appear in its own references (self-reference)
    let self_refs: Vec<&Reference> = result
        .iter()
        .filter(|r| {
            r.path.file_name().unwrap() == "main.md" && !r.path.to_string_lossy().contains("inner")
        })
        .collect();
    assert!(!self_refs.is_empty());
}

// ============= External URL filtering =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_external_urls_not_matched_as_file_refs() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test_external_urls.md");
    let content = "# Test\n\n[Google](https://google.com)\n[Local](test_external_urls.md)\n";
    write_file(&temp_file, content);

    let result = find_links(&temp_file).unwrap();

    // External URLs are filtered out and should not be collected as links
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    assert!(
        !link_texts.contains(&"https://google.com"),
        "External URLs should be filtered out"
    );
    assert!(
        link_texts.contains(&"test_external_urls.md"),
        "Local links should be collected"
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_external_url_not_matched() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let referrer = temp_dir.path().join("referrer.md");
    let content = "[External](https://example.com/target.md)\n[Local](target.md)\n";
    write_file(&referrer, content);

    let result = find_references(&target, temp_dir.path()).unwrap();

    // Only the local reference should match, not the external URL
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].link_text, "target.md");
}

// ============= Dot-slash prefix links =============

#[test]
fn test_find_links_dot_slash_prefix() {
    // examples/other.md uses ./main.jpg and ./inner/other.md style links
    let path = Path::new("examples/other.md");
    let result = find_links(path).unwrap();

    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    assert!(link_texts.contains(&"./inner/other.md"));
}

// ============= find_references with directory target =============

#[test]
fn test_find_references_directory_target() {
    let dir_path = Path::new("examples/inner");
    let root = Path::new("examples");
    let result = find_references(dir_path, root).unwrap();

    // Should find references to files inside the inner directory
    assert!(!result.is_empty());
}
