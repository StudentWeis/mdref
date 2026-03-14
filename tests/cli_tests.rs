use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Get the path to the compiled binary.
/// Assumes `cargo build` has been run before tests.
#[allow(clippy::unwrap_used)]
fn binary_path() -> std::path::PathBuf {
    // cargo test builds the binary in target/debug
    let mut path = std::env::current_exe().unwrap();
    // Navigate from test binary location to the workspace target/debug directory
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("mdref");
    path
}

#[allow(clippy::unwrap_used)]
fn write_file<P: AsRef<Path>>(path: P, content: &str) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

// ============= find command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_find_basic() {
    let binary = binary_path();
    if !binary.exists() {
        // Skip if binary not built yet (e.g., during cargo test --lib)
        return;
    }

    let output = Command::new(&binary)
        .args(["find", "examples/main.md", "--root", "examples"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("References to"));
    assert!(stdout.contains("Links in"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_find_nonexistent_file() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let output = Command::new(&binary)
        .args(["find", "nonexistent_file_xyz.md"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_find_no_references() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("lonely.md");
    write_file(&file, "# Lonely file");

    let output = Command::new(&binary)
        .args([
            "find",
            file.to_str().unwrap(),
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No references found"));
}

// ============= mv command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_basic() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("source.md");
    let target = temp_dir.path().join("target.md");
    write_file(&source, "# Source");

    let output = Command::new(&binary)
        .args([
            "mv",
            source.to_str().unwrap(),
            target.to_str().unwrap(),
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(target.exists());
    assert!(!source.exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_nonexistent_source() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output = Command::new(&binary)
        .args([
            "mv",
            temp_dir.path().join("ghost.md").to_str().unwrap(),
            temp_dir.path().join("target.md").to_str().unwrap(),
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error"));
}

// ============= rename command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_basic() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("old.md");
    write_file(&source, "# Old name");

    let output = Command::new(&binary)
        .args([
            "rename",
            source.to_str().unwrap(),
            "new.md",
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(temp_dir.path().join("new.md").exists());
    assert!(!source.exists());
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_nonexistent_source() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let output = Command::new(&binary)
        .args([
            "rename",
            temp_dir.path().join("ghost.md").to_str().unwrap(),
            "new.md",
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

// ============= version and help =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_version() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let output = Command::new(&binary).args(["--version"]).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mdref"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_help() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let output = Command::new(&binary).args(["--help"]).output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("find"));
    assert!(stdout.contains("mv"));
    assert!(stdout.contains("rename"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_no_args() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let output = Command::new(&binary).output().unwrap();

    // clap should print usage/help and exit with error
    assert!(!output.status.success());
}

// ============= End-to-end: mv with reference update =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_updates_references_e2e() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("doc.md");
    let ref_file = temp_dir.path().join("index.md");
    write_file(&source, "# Document");
    write_file(&ref_file, "See [doc](doc.md) for details.");

    let target = temp_dir.path().join("archive").join("doc.md");
    let output = Command::new(&binary)
        .args([
            "mv",
            source.to_str().unwrap(),
            target.to_str().unwrap(),
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify reference was updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("archive/doc.md"));
    assert!(!ref_content.contains("](doc.md)"));
}

// ============= End-to-end: rename with reference update =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_updates_references_e2e() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("old_doc.md");
    let ref_file = temp_dir.path().join("index.md");
    write_file(&source, "# Old Document");
    write_file(&ref_file, "See [doc](old_doc.md) for info.");

    let output = Command::new(&binary)
        .args([
            "rename",
            source.to_str().unwrap(),
            "new_doc.md",
            "--root",
            temp_dir.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("new_doc.md"));
    assert!(!ref_content.contains("old_doc.md"));
}
