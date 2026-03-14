use mdref::{MdrefError, find_links, find_references, mv_file};
use std::fs;
use std::io::Write;
use std::path::Path;

#[allow(clippy::unwrap_used)]
fn setup_test_dir(name: &str) -> String {
    let dir = format!("test_error_{}", name);
    if Path::new(&dir).exists() {
        fs::remove_dir_all(&dir).ok();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn teardown_test_dir(dir: &str) {
    if Path::new(dir).exists() {
        fs::remove_dir_all(dir).ok();
    }
}

#[allow(clippy::unwrap_used)]
fn write_file(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
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
    let dir = setup_test_dir("mv_io");
    let result = mv_file(
        &format!("{}/ghost.md", dir),
        &format!("{}/target.md", dir),
        &dir,
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MdrefError::Io(_)));

    teardown_test_dir(&dir);
}

// ============= mv_file overwrites existing target =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_target_already_exists() {
    let dir = setup_test_dir("overwrite");

    let source = format!("{}/source.md", dir);
    write_file(&source, "# Source content");

    let target = format!("{}/target.md", dir);
    write_file(&target, "# Old target content");

    // mv_file should overwrite the existing target
    let result = mv_file(&source, &target, &dir);
    assert!(result.is_ok());

    let content = fs::read_to_string(&target).unwrap();
    assert!(content.contains("Source content"));
    assert!(!content.contains("Old target content"));
    assert!(!Path::new(&source).exists());

    teardown_test_dir(&dir);
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
    let dir = setup_test_dir("empty_root");
    let target = format!("{}/target.md", dir);
    write_file(&target, "# Target");

    let result = find_references(&target, &dir).unwrap();
    // No other files reference target.md
    assert!(result.is_empty());

    teardown_test_dir(&dir);
}

// ============= Edge case: find_links on non-markdown returns empty =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_non_markdown_returns_empty() {
    let dir = setup_test_dir("non_md");
    let file = format!("{}/data.txt", dir);
    write_file(&file, "[This looks like a link](target.md)");

    let result = find_links(&file).unwrap();
    assert!(result.is_empty());

    teardown_test_dir(&dir);
}

// ============= Edge case: mv_file with no references =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_no_references_still_moves() {
    let dir = setup_test_dir("no_refs");
    let source = format!("{}/lonely.md", dir);
    write_file(&source, "# No one references me");

    let target = format!("{}/moved.md", dir);
    let result = mv_file(&source, &target, &dir);

    assert!(result.is_ok());
    assert!(Path::new(&target).exists());
    assert!(!Path::new(&source).exists());

    let content = fs::read_to_string(&target).unwrap();
    assert!(content.contains("No one references me"));

    teardown_test_dir(&dir);
}

// ============= Edge case: mv_file with no internal links =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_mv_file_no_internal_links() {
    let dir = setup_test_dir("no_links");
    let source = format!("{}/plain.md", dir);
    write_file(&source, "# Plain\n\nJust text, no links.");

    let ref_file = format!("{}/ref.md", dir);
    write_file(&ref_file, "[Plain](plain.md)");

    let target = format!("{}/sub/plain.md", dir);
    mv_file(&source, &target, &dir).unwrap();

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("sub/plain.md"));

    let target_content = fs::read_to_string(&target).unwrap();
    assert!(target_content.contains("Just text, no links."));

    teardown_test_dir(&dir);
}
