use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

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
fn setup_test_dir(name: &str) -> String {
    let dir = format!("test_cli_{}", name);
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

    let dir = setup_test_dir("find_no_refs");
    let file = format!("{}/lonely.md", dir);
    write_file(&file, "# Lonely file");

    let output = Command::new(&binary)
        .args(["find", &file, "--root", &dir])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No references found"));

    teardown_test_dir(&dir);
}

// ============= mv command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_basic() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let dir = setup_test_dir("mv_basic");
    let source = format!("{}/source.md", dir);
    let target = format!("{}/target.md", dir);
    write_file(&source, "# Source");

    let output = Command::new(&binary)
        .args(["mv", &source, &target, "--root", &dir])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(Path::new(&target).exists());
    assert!(!Path::new(&source).exists());

    teardown_test_dir(&dir);
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_nonexistent_source() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let dir = setup_test_dir("mv_nonexist");
    let output = Command::new(&binary)
        .args([
            "mv",
            &format!("{}/ghost.md", dir),
            &format!("{}/target.md", dir),
            "--root",
            &dir,
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error"));

    teardown_test_dir(&dir);
}

// ============= rename command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_basic() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let dir = setup_test_dir("rename_basic");
    let source = format!("{}/old.md", dir);
    write_file(&source, "# Old name");

    let output = Command::new(&binary)
        .args(["rename", &source, "new.md", "--root", &dir])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(Path::new(&format!("{}/new.md", dir)).exists());
    assert!(!Path::new(&source).exists());

    teardown_test_dir(&dir);
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_nonexistent_source() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let dir = setup_test_dir("rename_nonexist");
    let output = Command::new(&binary)
        .args([
            "rename",
            &format!("{}/ghost.md", dir),
            "new.md",
            "--root",
            &dir,
        ])
        .output()
        .unwrap();

    assert!(!output.status.success());

    teardown_test_dir(&dir);
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

    let dir = setup_test_dir("mv_e2e");
    let source = format!("{}/doc.md", dir);
    let ref_file = format!("{}/index.md", dir);
    write_file(&source, "# Document");
    write_file(&ref_file, "See [doc](doc.md) for details.");

    let target = format!("{}/archive/doc.md", dir);
    let output = Command::new(&binary)
        .args(["mv", &source, &target, "--root", &dir])
        .output()
        .unwrap();

    assert!(output.status.success());

    // Verify reference was updated
    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("archive/doc.md"));
    assert!(!ref_content.contains("](doc.md)"));

    teardown_test_dir(&dir);
}

// ============= End-to-end: rename with reference update =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_updates_references_e2e() {
    let binary = binary_path();
    if !binary.exists() {
        return;
    }

    let dir = setup_test_dir("rename_e2e");
    let source = format!("{}/old_doc.md", dir);
    let ref_file = format!("{}/index.md", dir);
    write_file(&source, "# Old Document");
    write_file(&ref_file, "See [doc](old_doc.md) for info.");

    let output = Command::new(&binary)
        .args(["rename", &source, "new_doc.md", "--root", &dir])
        .output()
        .unwrap();

    assert!(output.status.success());

    let ref_content = fs::read_to_string(&ref_file).unwrap();
    assert!(ref_content.contains("new_doc.md"));
    assert!(!ref_content.contains("old_doc.md"));

    teardown_test_dir(&dir);
}
