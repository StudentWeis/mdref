use std::{fs, io::Write, path::Path};

use mdref::{MdrefError, NoopProgress, Reference, find_links, find_references};
use rstest::rstest;
use tempfile::TempDir;

mod common;

use common::{fixture_multi_file_reference, fixture_unicode_paths, write_file};

// Library tests for `find_*` focus on core parsing and filesystem behavior.
// Output formatting and process exit behavior belong to CLI tests.

// ============= find_links error handling tests =============

/// find_links should return an error when the file does not exist.
#[test]
fn test_find_links_returns_error_for_nonexistent_file() {
    let result = find_links(Path::new("nonexistent.md"));
    match result {
        Err(MdrefError::IoRead { path, source }) => {
            assert!(path.ends_with("nonexistent.md"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected io read error for nonexistent file, got {other:?}"),
    }
}

/// find_links should fail fast for invalid UTF-8 markdown input.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_invalid_utf8_input_returns_invalid_data_error() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("invalid.md");
    fs::write(&path, b"# Invalid\xFF\n").unwrap();

    let result = find_links(&path);

    match result {
        Err(MdrefError::IoRead { path, source }) => {
            assert!(path.ends_with("invalid.md"));
            assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
        }
        other => panic!("expected io read error for invalid utf-8, got {other:?}"),
    }
}

/// find_links should return an empty Vec for non-markdown files.
#[test]
fn test_find_links_returns_empty_for_non_markdown_file() {
    let path = Path::new("Cargo.toml");
    let result = find_links(path).unwrap();
    assert!(
        result.is_empty(),
        "Non-markdown files should return empty Vec"
    );
}

// ============= find_links core behavior tests =============

/// find_links should correctly extract all markdown links from a file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_extracts_all_links() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    // Create file with 3 markdown links and 2 image links
    write_file(
        &temp_file,
        "# Test\n\n[link1](a.md) [link2](b.md)\n\n![img](img.png) ![img2](img2.png)\n",
    );

    let result = find_links(&temp_file).unwrap();
    assert_eq!(
        result.len(),
        4,
        "Should find 4 links (2 markdown + 2 image)"
    );
}

/// find_links should correctly identify link text and positions.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_returns_correct_link_texts() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(
        &temp_file,
        "# Test\n\n[First](first.md) [Second](second.md)\n",
    );

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(link_texts.contains(&"first.md"), "Should contain first.md");
    assert!(
        link_texts.contains(&"second.md"),
        "Should contain second.md"
    );
}

/// find_links should return correct line and column numbers for links.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_returns_correct_positions() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(&temp_file, "# Test\n\n[link](target.md)\n");

    let result = find_links(&temp_file).unwrap();
    assert_eq!(result.len(), 1, "Should find exactly one link");

    let reference = &result[0];
    assert!(reference.line > 0, "Line number should be greater than 0");
    assert!(
        reference.column > 0,
        "Column number should be greater than 0"
    );
}

/// find_links should handle multiple links on the same line correctly.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_handles_multiple_links_on_same_line() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    // Line 3 has 3 links
    write_file(
        &temp_file,
        "# Test\n\n[one](a.md) [two](b.md) [three](c.md)\n",
    );

    let result = find_links(&temp_file).unwrap();

    // All three links should be on line 3
    let line3_links: Vec<&Reference> = result.iter().filter(|r| r.line == 3).collect();
    assert_eq!(line3_links.len(), 3, "Should find 3 links on line 3");

    // Column numbers should be distinct and increasing
    let mut columns: Vec<usize> = line3_links.iter().map(|r| r.column).collect();
    columns.sort();
    assert!(
        columns.windows(2).all(|w| w[0] < w[1]),
        "Column numbers should be increasing"
    );
}

/// find_links should return an empty Vec for an empty markdown file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_returns_empty_for_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("empty.md");
    fs::File::create(&temp_file)
        .unwrap()
        .write_all(b"")
        .unwrap();

    let result = find_links(&temp_file).unwrap();
    assert_eq!(result.len(), 0, "Empty file should have no links");
}

// ============= Image link tests =============

/// find_links should include image links in the results.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_includes_image_links() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(&temp_file, "# Test\n\n![Alt text](image.png)\n");

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(
        link_texts.contains(&"image.png"),
        "Should include image links"
    );
}

/// find_links should handle image links with ./ prefix.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_handles_image_with_dot_slash_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(&temp_file, "# Test\n\n![Image](./image.png)\n");

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(
        link_texts.contains(&"./image.png"),
        "Should preserve ./ prefix in image links"
    );
}

// ============= External URL filtering tests =============

/// find_links should filter out external URLs (http/https).
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_filters_external_urls() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(
        &temp_file,
        "# Test\n\n[Google](https://google.com)\n[Local](local.md)\n",
    );

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(
        !link_texts.contains(&"https://google.com"),
        "External URLs should be filtered out"
    );
    assert!(
        link_texts.contains(&"local.md"),
        "Local links should be included"
    );
}

/// find_links should filter out other URL schemes (ftp, mailto, etc).
#[rstest]
#[case::http("http://example.com/doc.md")]
#[case::ftp("ftp://example.com/file.md")]
#[case::mailto("mailto:user@example.com")]
#[allow(clippy::unwrap_used)]
fn test_find_links_filters_url_schemes(#[case] url: &str) {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    let content = format!("# Test\n\n[Link]({url})\n");
    write_file(&temp_file, &content);

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(
        !link_texts.contains(&url),
        "URL scheme {url} should be filtered out"
    );
}

// ============= Dot-slash prefix tests =============

/// find_links should preserve ./ prefix in markdown links.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_preserves_dot_slash_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let temp_file = temp_dir.path().join("test.md");
    write_file(&temp_file, "# Test\n\n[Link](./other.md)\n");

    let result = find_links(&temp_file).unwrap();
    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();

    assert!(
        link_texts.contains(&"./other.md"),
        "Should preserve ./ prefix"
    );
}

// ============= find_references core behavior tests =============

/// find_references should find all files that reference the target file.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_finds_referencing_files() {
    let fixture = fixture_multi_file_reference();

    let result = find_references(&fixture.target, &fixture.root, &NoopProgress).unwrap();
    assert_eq!(result.len(), 3, "Should find 3 files referencing target.md");
}

/// find_references should return an error when target file does not exist.
#[test]
fn test_find_references_returns_error_for_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let result = find_references(
        temp_dir.path().join("ghost.md"),
        temp_dir.path(),
        &NoopProgress,
    );

    match result {
        Err(MdrefError::IoRead { path, source }) => {
            assert!(path.ends_with("ghost.md"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected io read error for nonexistent file, got {other:?}"),
    }
}

/// find_references should return an error instead of silently skipping invalid UTF-8 markdown files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_invalid_utf8_file_returns_invalid_data_error() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let invalid_ref = temp_dir.path().join("invalid.md");
    fs::write(&invalid_ref, b"[Broken](target.md)\xFF").unwrap();

    let result = find_references(&target, temp_dir.path(), &NoopProgress);

    match result {
        Err(MdrefError::IoRead { path, source }) => {
            assert!(path.ends_with("invalid.md"));
            assert_eq!(source.kind(), std::io::ErrorKind::InvalidData);
        }
        other => panic!("expected io read error for invalid utf-8, got {other:?}"),
    }
}

/// find_references should return empty Vec when no references exist.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_returns_empty_when_no_references() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let other = temp_dir.path().join("other.md");
    write_file(&other, "# Other\n\nNo references here");

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();
    assert_eq!(
        result.len(),
        0,
        "Should return empty Vec when no references"
    );
}

/// find_references should only return markdown files.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_only_returns_markdown_files() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    // Create markdown and non-markdown files referencing target
    let md_ref = temp_dir.path().join("ref.md");
    write_file(&md_ref, "[link](target.md)");

    let txt_ref = temp_dir.path().join("ref.txt");
    write_file(&txt_ref, "See target.md");

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();

    for reference in &result {
        assert_eq!(
            reference.path.extension().and_then(|s| s.to_str()),
            Some("md"),
            "All results should be markdown files"
        );
    }
}

// ============= find_references with nested directories =============

/// find_references should find references from nested directories.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_finds_nested_references() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("docs").join("target.md");
    write_file(&target, "# Target");

    // Reference from root level
    let root_ref = temp_dir.path().join("index.md");
    write_file(&root_ref, "See [docs](docs/target.md)");

    // Reference from sibling level
    let sibling_ref = temp_dir.path().join("other.md");
    write_file(&sibling_ref, "Check [docs](docs/target.md)");

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();
    assert!(
        result.len() >= 2,
        "Should find references from different directory levels"
    );
}

/// find_references should skip markdown files ignored by .gitignore.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_respects_gitignore() {
    let temp_dir = TempDir::new().unwrap();

    write_file(temp_dir.path().join(".gitignore"), "ignored/\n");

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let visible_ref = temp_dir.path().join("ref.md");
    write_file(&visible_ref, "[Visible](target.md)");

    let ignored_ref = temp_dir.path().join("ignored").join("ref.md");
    write_file(&ignored_ref, "[Ignored](../target.md)");

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();

    assert_eq!(result.len(), 1, "Ignored markdown files should be skipped");
    assert_eq!(result[0].path, visible_ref);
}

/// find_references should handle target as a directory.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_handles_directory_target() {
    let temp_dir = TempDir::new().unwrap();

    // Create files in a directory
    let dir = temp_dir.path().join("docs");
    let file1 = dir.join("file1.md");
    let file2 = dir.join("file2.md");
    write_file(&file1, "# File 1");
    write_file(&file2, "# File 2");

    // Create reference to a file in the directory
    let ref_file = temp_dir.path().join("index.md");
    write_file(&ref_file, "See [file1](docs/file1.md)");

    let result = find_references(&dir, temp_dir.path(), &NoopProgress).unwrap();
    assert!(
        !result.is_empty(),
        "Should find references to files in the directory"
    );
}

// ============= Self-reference tests =============

/// find_references should detect self-references (file referencing itself).
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_detects_self_reference() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("self.md");
    write_file(&target, "# Self\n\n[Self link](self.md)");

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();

    let self_refs: Vec<&Reference> = result
        .iter()
        .filter(|r| r.path.file_name().unwrap() == "self.md")
        .collect();

    assert!(!self_refs.is_empty(), "Should detect self-references");
}

// ============= External URL not matched as reference =============

/// find_references should not match external URLs as references.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_ignores_external_urls() {
    let temp_dir = TempDir::new().unwrap();

    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(
        &ref_file,
        "[External](https://example.com/target.md)\n[Local](target.md)",
    );

    let result = find_references(&target, temp_dir.path(), &NoopProgress).unwrap();

    // Only the local reference should match
    assert_eq!(
        result.len(),
        1,
        "External URLs should not be matched as references"
    );
    assert_eq!(result[0].link_text, "target.md");
}

// ============= Unicode tests =============

/// Test find_links with Unicode filenames (Chinese, Japanese, Korean, emoji).
#[rstest]
#[case::chinese("中文文档.md", "其他文档.md")]
#[case::japanese("ドキュメント.md", "他のファイル.md")]
#[case::korean("한글.md", "다른파일.md")]
#[case::emoji("📝笔记.md", "📎附件.md")]
#[allow(clippy::unwrap_used)]
fn test_find_links_handles_unicode_filenames(#[case] filename: &str, #[case] link: &str) {
    let temp_dir = TempDir::new().unwrap();
    let unicode_file = temp_dir.path().join(filename);
    write_file(&unicode_file, &format!("# Test\n\n[Link]({link})\n"));

    let result = find_links(&unicode_file).unwrap();
    assert_eq!(
        result.len(),
        1,
        "Should find exactly one link in Unicode-named file"
    );
    assert_eq!(result[0].link_text, link);
}

/// Test find_links with mixed Unicode content.
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_links_handles_mixed_unicode_content() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("mixed.md");
    write_file(
        &file,
        "# Mixed 混合 ミックス\n\n[中文](中文.md) [日本語](日本語.md) [한글](한글.md)\n",
    );

    let result = find_links(&file).unwrap();
    assert_eq!(result.len(), 3, "Should find 3 Unicode links");

    let link_texts: Vec<&str> = result.iter().map(|r| r.link_text.as_str()).collect();
    assert!(link_texts.contains(&"中文.md"));
    assert!(link_texts.contains(&"日本語.md"));
    assert!(link_texts.contains(&"한글.md"));
}

/// Test find_references with Unicode file and directory paths.
#[rstest]
#[case::unicode_file(
    "目标文件.md",
    "# 目标",
    "引用者.md",
    "请参考 [目标文件](目标文件.md)",
    "目标文件.md"
)]
#[case::unicode_directory(
    "文档库/参考资料.md",
    "# 参考资料",
    "索引.md",
    "查看 [参考资料](文档库/参考资料.md)",
    "文档库/参考资料.md"
)]
#[allow(clippy::unwrap_used)]
fn test_find_references_handles_unicode_paths(
    #[case] target_relative_path: &str,
    #[case] target_content: &str,
    #[case] reference_name: &str,
    #[case] reference_content: &str,
    #[case] expected_link_text: &str,
) {
    let fixture = fixture_unicode_paths();
    let target = fixture.root.join(target_relative_path);
    write_file(&target, target_content);

    let ref_file = fixture.root.join(reference_name);
    write_file(&ref_file, reference_content);

    let result = find_references(&target, &fixture.root, &NoopProgress).unwrap();
    assert_eq!(result.len(), 1, "Should find one Unicode reference");
    assert_eq!(result[0].link_text, expected_link_text);
}

// ============= Path normalization tests =============

/// Regression test for #8.
///
/// When `root` is `"."`, `find_references` should return `Reference.path` values
/// without a `./` prefix — they should match the shape a user would type on the
/// command line (e.g. `sub/ref.md`, not `./sub/ref.md`).
#[test]
#[allow(clippy::unwrap_used)]
fn test_find_references_paths_have_no_dot_slash_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("target.md");
    write_file(&target, "# Target");

    let sub_dir = temp_dir.path().join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    let ref_file = sub_dir.join("ref.md");
    write_file(&ref_file, "[Link](../target.md)");

    // Use "." as root — this is the shape that triggers WalkBuilder's ./ prefix.
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let result = find_references("target.md", ".", &NoopProgress).unwrap();

    std::env::set_current_dir(&original_dir).unwrap();

    assert_eq!(result.len(), 1, "Should find exactly one reference");
    let ref_path = result[0].path.to_string_lossy();
    assert!(
        !ref_path.starts_with("./"),
        "Reference path should not have ./ prefix, got: {ref_path}"
    );
    assert_eq!(
        ref_path, "sub/ref.md",
        "Reference path should be a clean relative path"
    );
}
