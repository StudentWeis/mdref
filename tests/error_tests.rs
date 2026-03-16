use mdref::{MdrefError, find_links, find_references, mv_file};
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

// ============= IO error tests =============

#[test]
fn test_find_links_io_error_nonexistent_file() {
    let result = find_links(Path::new("absolutely_nonexistent_file.md"));
    assert!(result.is_err());

    let error = result.unwrap_err();
    assert!(matches!(error, MdrefError::Io(_)));
}

#[test]
fn test_find_references_io_error_nonexistent_file() {
    let result = find_references(
        Path::new("absolutely_nonexistent_file.md"),
        Path::new("examples"),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MdrefError::Io(_)));
}

#[test]
fn test_mv_file_io_error_nonexistent_source() {
    let temp_dir = TempDir::new().unwrap();
    let result = mv_file(
        temp_dir.path().join("ghost.md").to_str().unwrap(),
        temp_dir.path().join("target.md").to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
    );
    assert!(result.is_err());
    // mv_file now explicitly checks for source existence and returns Path error
    // with a descriptive message before attempting any IO operations.
    assert!(matches!(result.unwrap_err(), MdrefError::Path(_)));
}

// ============= mv_file rejects existing target =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_target_already_exists() {
    let temp_dir = TempDir::new().unwrap();

    let source = temp_dir.path().join("source.md");
    write_file(source.clone(), "# Source content");

    let target = temp_dir.path().join("target.md");
    write_file(target.clone(), "# Old target content");

    // mv_file should return an error when target already exists
    let result = mv_file(
        source.to_str().unwrap(),
        target.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
    );
    assert!(result.is_err());

    // Target content should remain unchanged
    let content = fs::read_to_string(&target).unwrap();
    assert!(content.contains("Old target content"));
    assert!(!content.contains("Source content"));
    // Source should still exist
    assert!(source.exists());
}

// ============= Error display tests =============

#[test]
fn test_mdref_error_io_display() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let error = MdrefError::Io(io_error);
    let display = format!("{}", error);
    assert!(display.contains("IO error"));
    assert!(display.contains("file not found"));
}

#[test]
fn test_mdref_error_path_display() {
    let error = MdrefError::Path("No parent directory".to_string());
    let display = format!("{}", error);
    assert!(display.contains("Path error"));
    assert!(display.contains("No parent directory"));
}

#[test]
fn test_mdref_error_invalid_line_display() {
    let error = MdrefError::InvalidLine("Line 999 out of range".to_string());
    let display = format!("{}", error);
    assert!(display.contains("Invalid line number"));
    assert!(display.contains("999"));
}

// ============= Edge case: empty root directory =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_empty_root_directory() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("target.md");
    write_file(target.clone(), "# Target");

    let result = find_references(&target, temp_dir.path()).unwrap();
    // No other files reference target.md
    assert!(result.is_empty());
}

// ============= Edge case: find_links on non-markdown returns empty =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_non_markdown_returns_empty() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("data.txt");
    write_file(file.clone(), "[This looks like a link](target.md)");

    let result = find_links(&file).unwrap();
    assert!(result.is_empty());
}

// ============= Edge case: mv_file with no references =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_no_references_still_moves() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("lonely.md");
    write_file(source.clone(), "# No one references me");

    let target = temp_dir.path().join("moved.md");
    let result = mv_file(
        source.to_str().unwrap(),
        target.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
    );

    assert!(result.is_ok());
    assert!(target.exists());
    assert!(!source.exists());

    let content = fs::read_to_string(&target).unwrap();
    assert!(content.contains("No one references me"));
}

// ============= Edge case: mv_file with no internal links =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_no_internal_links() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("plain.md");
    write_file(source.clone(), "# Plain\n\nJust text, no links.");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(ref_file.clone(), "[Plain](plain.md)");

    let target = temp_dir.path().join("sub").join("plain.md");
    mv_file(
        source.to_str().unwrap(),
        target.to_str().unwrap(),
        temp_dir.path().to_str().unwrap(),
        false,
    )
    .unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("sub/plain.md"));

    let target_content = fs::read_to_string(&target).unwrap();
    assert!(target_content.contains("Just text, no links."));
}
