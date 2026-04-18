use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use crate::{MdrefError, Result, core::pathdiff::diff_paths};

/// Collect markdown files while respecting ignore files such as `.gitignore`.
pub fn collect_markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut builder = WalkBuilder::new(root);
    builder.standard_filters(true).require_git(false);

    builder
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
        })
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .collect()
}

/// Strip the UTF-8 BOM (Byte Order Mark) prefix from a line.
///
/// UTF-8 BOM is the character U+FEFF, which may appear at the start of a file.
/// Returns a tuple of (stripped_line, bom_length) where bom_length is 0 if no BOM.
pub fn strip_utf8_bom_prefix(line: &str) -> (&str, usize) {
    match line.strip_prefix('\u{feff}') {
        Some(stripped) => (stripped, '\u{feff}'.len_utf8()),
        None => (line, 0),
    }
}

/// Decode URL percent-encoded characters in a link.
///
/// This function decodes common URL encodings like `%20` (space), `%2B` (+), etc.
/// This is necessary because Markdown links may contain URL-encoded paths,
/// especially when the actual file has spaces or special characters.
///
/// # Examples
/// - `"my%20file.md"` -> `"my file.md"`
/// - `"path%20to/file.md"` -> `"path to/file.md"`
/// - `"file.md#section"` -> `"file.md#section"` (anchor preserved)
pub fn url_decode_link(link: &str) -> String {
    let mut result = String::with_capacity(link.len());
    let bytes = link.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // Collect consecutive %XX sequences into a byte buffer
            let mut decoded_bytes = Vec::new();
            while i < bytes.len() && bytes[i] == b'%' && i + 2 < bytes.len() {
                let hex = &link[i + 1..i + 3];
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        decoded_bytes.push(byte);
                        i += 3;
                    }
                    Err(_) => break,
                }
            }

            if decoded_bytes.is_empty() {
                // Invalid %XX sequence, push '%' as-is and advance
                result.push('%');
                i += 1;
            } else {
                match String::from_utf8(decoded_bytes) {
                    Ok(decoded) => result.push_str(&decoded),
                    Err(err) => {
                        result.push_str(&String::from_utf8_lossy(err.as_bytes()));
                    }
                }
            }
        } else {
            // Non-encoded byte: find the full UTF-8 character starting at this position
            // and push it as a whole char to preserve multibyte Unicode characters
            let remaining = &link[i..];
            let ch = remaining.chars().next().unwrap_or('\0');
            result.push(ch);
            i += ch.len_utf8();
        }
    }

    result
}

/// Check if a link is an external URL (http, https, ftp, mailto, etc.)
///
/// Returns `true` for:
/// - URLs with scheme containing `://` (http://, https://, ftp://, etc.) except `file://`
/// - Common URI schemes without `://` (mailto:, tel:, data:, javascript:)
///
/// Returns `false` for:
/// - Local file paths (relative or absolute)
/// - `file://` URLs (these are local files)
/// - Pure anchor links (handled separately)
pub fn is_external_url(link: &str) -> bool {
    let link_lower = link.to_lowercase();

    // Handle schemes without :// (these are external)
    // mailto:, tel:, data:, javascript:, etc.
    let non_colon_slash_schemes = [
        "mailto:",
        "tel:",
        "sms:",
        "data:",
        "javascript:",
        "vbscript:",
        "about:",
        "blob:",
        "geo:",
        "irc:",
        "ircs:",
        "magnet:",
        "mms:",
        "news:",
        "nntp:",
        "sip:",
        "sips:",
        "skype:",
        "ssh:",
        "telnet:",
        "urn:",
        "webcal:",
    ];

    for scheme in &non_colon_slash_schemes {
        if link_lower.starts_with(scheme) {
            return true;
        }
    }

    // file:// is a local file, not external
    if link_lower.starts_with("file://") {
        return false;
    }

    // Check for :// schemes (http://, https://, ftp://, etc.)
    if link.contains("://") {
        // Extract scheme and check it's not a local file scheme
        if let Some(scheme_end) = link.find("://") {
            let scheme = &link[..scheme_end].to_lowercase();
            // Only known remote schemes are external
            let remote_schemes = [
                "http", "https", "ftp", "ftps", "sftp", "ws", "wss", "git", "svn", "svn+ssh", "hg",
                "cvs", "apt", "dav", "nfs", "smb",
            ];
            return remote_schemes.contains(&scheme.as_str());
        }
    }

    false
}

/// Strip the anchor (fragment) from a link URL.
/// For example, "file.md#section" becomes "file.md", and "#section" returns None.
pub fn strip_anchor(link: &str) -> Option<&str> {
    // Handle pure anchor links (e.g., "#section") - these are internal to the file
    if link.starts_with('#') {
        return None;
    }

    // Split on '#' and take the part before it
    link.split('#').next()
}

/// Compute the relative path from one file to another.
/// Handles the case where either path may not exist yet (e.g. during dry-run)
/// by canonicalizing parent directories when possible and falling back to raw paths.
pub fn relative_path(from: &Path, to: &Path) -> Result<PathBuf> {
    let to_resolved = resolve_path(to)?;
    let from_parent = from.parent().ok_or_else(|| MdrefError::PathValidation {
        path: from.to_path_buf(),
        details: "no parent directory".to_string(),
    })?;
    let from_resolved = if from_parent.exists() {
        from_parent.canonicalize()?
    } else {
        resolve_parent(from_parent)?
    };
    Ok(diff_paths(to_resolved, from_resolved).unwrap_or_default())
}

/// Resolve a path to its canonical form, handling non-existent files
/// by canonicalizing the nearest existing ancestor and joining the rest.
pub fn resolve_path(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(path.canonicalize()?);
    }
    let parent = path.parent().ok_or_else(|| MdrefError::PathValidation {
        path: path.to_path_buf(),
        details: "no parent directory".to_string(),
    })?;
    let filename = path.file_name().ok_or_else(|| MdrefError::PathValidation {
        path: path.to_path_buf(),
        details: "no file name".to_string(),
    })?;
    let parent_resolved = if parent.exists() {
        parent.canonicalize()?
    } else {
        resolve_parent(parent)?
    };
    Ok(parent_resolved.join(filename))
}

/// Resolve a directory path by canonicalizing the nearest existing ancestor
/// and joining the remaining non-existent components.
pub fn resolve_parent(dir: &Path) -> Result<PathBuf> {
    let mut components_to_append = Vec::new();
    let mut current = dir;
    loop {
        if current.exists() {
            let mut resolved = current.canonicalize()?;
            for component in components_to_append.into_iter().rev() {
                resolved.push(component);
            }
            return Ok(resolved);
        }
        if let Some(file_name) = current.file_name() {
            components_to_append.push(file_name.to_owned());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(dir.to_path_buf()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rstest::rstest;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_is_external_url() {
        assert!(is_external_url("http://example.com"));
        assert!(is_external_url("http://localhost:8080/path"));
        assert!(is_external_url("https://google.com"));
        assert!(is_external_url("https://github.com/user/repo"));
        assert!(is_external_url("ftp://files.example.com/doc.md"));
        assert!(is_external_url("sftp://secure.example.com/file"));
        assert!(is_external_url("git://github.com/user/repo.git"));
    }

    #[test]
    fn test_is_external_url_local_paths() {
        assert!(!is_external_url("local.md"));
        assert!(!is_external_url("sub/dir/file.md"));
        assert!(!is_external_url("../parent/file.md"));
        assert!(!is_external_url("./relative.md"));
        assert!(!is_external_url("image.png"));
    }

    #[test]
    fn test_is_external_url_file_scheme() {
        // file:// should NOT be considered external (it's a local file)
        assert!(!is_external_url("file:///path/to/file.md"));
        assert!(!is_external_url("file://localhost/path/to/file.md"));
        assert!(!is_external_url("FILE:///C:/Users/doc.md")); // case insensitive
    }

    #[test]
    fn test_is_external_url_schemes_without_colon_slash() {
        // Schemes without :// should be detected as external
        assert!(is_external_url("mailto:user@example.com"));
        assert!(is_external_url("tel:+1234567890"));
        assert!(is_external_url("sms:+1234567890"));
        assert!(is_external_url("data:text/plain,Hello"));
        assert!(is_external_url("javascript:alert('xss')"));
        assert!(is_external_url("MAILTO:user@example.com")); // case insensitive
    }

    #[test]
    fn test_is_external_url_windows_paths() {
        // Windows absolute paths should NOT be external
        assert!(!is_external_url("C:\\path\\to\\file.md"));
        assert!(!is_external_url("D:/path/to/file.md"));
        assert!(!is_external_url("C:/Users/name/documents/readme.md"));
    }

    // ============= strip_anchor tests =============

    #[test]
    fn test_strip_anchor_with_anchor() {
        assert_eq!(strip_anchor("file.md#section"), Some("file.md"));
        assert_eq!(strip_anchor("doc.md#details"), Some("doc.md"));
    }

    #[test]
    fn test_strip_anchor_without_anchor() {
        assert_eq!(strip_anchor("file.md"), Some("file.md"));
        assert_eq!(strip_anchor("path/to/doc.md"), Some("path/to/doc.md"));
    }

    #[test]
    fn test_strip_anchor_pure_anchor() {
        assert_eq!(strip_anchor("#section"), None);
        assert_eq!(strip_anchor("#table-of-contents"), None);
    }

    // ============= relative_path tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_same_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("to.md");
        fs::write(&from, "").unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("sub").join("from.md");
        let to = temp_dir.path().join("to.md");
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        fs::write(&from, "").unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_child_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        let to = temp_dir.path().join("sub").join("to.md");
        fs::write(&from, "").unwrap();
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("sub/to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_deep_cross_directory() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("a").join("b").join("from.md");
        let to = temp_dir.path().join("x").join("y").join("to.md");
        fs::create_dir_all(from.parent().unwrap()).unwrap();
        fs::write(&from, "").unwrap();
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, "").unwrap();

        let result = relative_path(&from, &to).unwrap();
        assert_eq!(result, PathBuf::from("../../x/y/to.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_relative_path_nonexistent_target() {
        let temp_dir = TempDir::new().unwrap();
        let from = temp_dir.path().join("from.md");
        fs::write(&from, "").unwrap();

        let ghost = temp_dir.path().join("ghost.md");
        let result = relative_path(&from, &ghost).unwrap();
        assert_eq!(result, PathBuf::from("ghost.md"));
    }

    // ============= resolve_path tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_path_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("file.md");
        fs::write(&file, "").unwrap();

        let result = resolve_path(&file).unwrap();
        assert!(result.ends_with("file.md"));
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_path_nonexistent_file_in_existing_dir() {
        let temp_dir = TempDir::new().unwrap();
        let file = temp_dir.path().join("ghost.md");

        let result = resolve_path(&file).unwrap();
        assert!(result.ends_with("ghost.md"));
    }

    // ============= resolve_parent tests =============

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_parent_existing_dir() {
        let temp_dir = TempDir::new().unwrap();

        let result = resolve_parent(temp_dir.path()).unwrap();
        assert!(result.exists());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_resolve_parent_nonexistent_dir() {
        let temp_dir = TempDir::new().unwrap();
        let nonexist = temp_dir.path().join("a").join("b").join("c");

        let result = resolve_parent(&nonexist).unwrap();
        // Should resolve to the temp_dir + "a/b/c"
        assert!(result.ends_with("a/b/c"));
    }

    // ============= url_decode_link tests =============

    #[test]
    fn test_url_decode_link_no_encoding() {
        // Plain path without any encoding should remain unchanged
        assert_eq!(url_decode_link("file.md"), "file.md");
        assert_eq!(url_decode_link("path/to/file.md"), "path/to/file.md");
        assert_eq!(url_decode_link("./relative.md"), "./relative.md");
    }

    #[test]
    fn test_url_decode_link_space_encoding() {
        // %20 should be decoded to space
        assert_eq!(url_decode_link("my%20file.md"), "my file.md");
        assert_eq!(url_decode_link("path%20to/file.md"), "path to/file.md");
        assert_eq!(url_decode_link("my%20file%20name.md"), "my file name.md");
    }

    #[test]
    fn test_url_decode_link_multiple_spaces() {
        // Multiple encoded spaces
        assert_eq!(
            url_decode_link("docs/my%20document%20with%20spaces.md"),
            "docs/my document with spaces.md"
        );
    }

    #[test]
    fn test_url_decode_link_special_chars() {
        // Other URL encoded characters
        assert_eq!(url_decode_link("file%2Bname.md"), "file+name.md"); // +
        assert_eq!(url_decode_link("file%23name.md"), "file#name.md"); // #
        assert_eq!(url_decode_link("file%26name.md"), "file&name.md"); // &
        assert_eq!(url_decode_link("file%40name.md"), "file@name.md"); // @
    }

    #[test]
    fn test_url_decode_link_preserves_anchor() {
        // Anchor (fragment) should be preserved, only path decoded
        assert_eq!(
            url_decode_link("my%20file.md#section"),
            "my file.md#section"
        );
        assert_eq!(
            url_decode_link("path%20to/file.md#my-section"),
            "path to/file.md#my-section"
        );
    }

    #[test]
    fn test_url_decode_link_invalid_encoding() {
        // Invalid encoding should be handled gracefully
        // % without two hex digits
        assert_eq!(url_decode_link("file%.md"), "file%.md");
        assert_eq!(url_decode_link("file%2.md"), "file%2.md");
        // Incomplete encoding at end
        assert_eq!(url_decode_link("file%2"), "file%2");
    }

    #[test]
    fn test_url_decode_link_mixed_encoding() {
        // Mix of encoded and unencoded characters
        assert_eq!(url_decode_link("my%20file name.md"), "my file name.md");
    }

    #[test]
    fn test_url_decode_link_percent_sign() {
        // %25 is encoded percent sign
        assert_eq!(url_decode_link("100%25.md"), "100%.md");
    }

    #[rstest]
    #[case::chinese_single("%E4%B8%AD.md", "中.md")]
    #[case::chinese_filename("%E4%B8%AD%E6%96%87%E6%96%87%E6%A1%A3.md", "中文文档.md")]
    #[case::japanese("docs/%E6%97%A5%E6%9C%AC%E8%AA%9E.md", "docs/日本語.md")]
    #[case::emoji("%F0%9F%93%9D.md", "📝.md")]
    #[case::mixed_ascii_and_chinese("file_%E4%B8%AD.md", "file_中.md")]
    #[case::space_and_chinese("my%20%E6%96%87%E4%BB%B6.md", "my 文件.md")]
    fn test_url_decode_link_multibyte_utf8_decodes_correctly(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(url_decode_link(input), expected);
    }
}
