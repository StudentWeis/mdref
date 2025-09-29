use mdref::{find_links, find_references};

#[test]
fn test_find_links_nonexistent_file() {
    let result = find_links(std::path::Path::new("nonexistent.md"));
    assert!(result.is_err());
}

#[test]
fn test_find_links_basic() {
    let path = std::path::Path::new("examples/main.md");
    let result = find_links(path).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_find_references_basic() {
    let path = std::path::Path::new("examples/main.md");
    let result = find_references(path, path.parent().unwrap()).unwrap();
    assert_eq!(result.len(), 6)
}
