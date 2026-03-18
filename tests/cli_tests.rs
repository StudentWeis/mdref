mod common;

use common::{read_file, run_cli, temp_dir, write_file};

// CLI tests only cover process-level contracts: argument wiring, exit codes,
// stdout/stderr output, and one representative end-to-end flow per command.
// Core path rewriting and error-branch behavior stays in lib_* and error tests.

// ============= find command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_find_basic() {
    let output = run_cli(&["find", "examples/main.md", "--root", "examples"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("References to"));
    assert!(stdout.contains("Links in"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_find_no_references() {
    let temp_dir = temp_dir();
    let file = temp_dir.path().join("lonely.md");
    write_file(&file, "# Lonely file");

    let output = run_cli(&[
        "find",
        file.to_str().unwrap(),
        "--root",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No references found"));
}

// ============= mv command tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_runtime_error_reports_exit_code_and_stderr() {
    let temp_dir = temp_dir();
    let output = run_cli(&[
        "mv",
        temp_dir.path().join("ghost.md").to_str().unwrap(),
        temp_dir.path().join("target.md").to_str().unwrap(),
        "--root",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Error:"));
}

// ============= rename command tests =============

// ============= version and help =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_version() {
    let output = run_cli(&["--version"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("mdref"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_help() {
    let output = run_cli(&["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("find"));
    assert!(stdout.contains("mv"));
    assert!(stdout.contains("rename"));
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_no_args() {
    let output = run_cli(&[]);

    // clap should print usage/help and exit with error
    assert!(!output.status.success());
}

// ============= End-to-end: mv with reference update =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_updates_references_e2e() {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join("doc.md");
    let ref_file = temp_dir.path().join("index.md");
    write_file(&source, "# Document");
    write_file(&ref_file, "See [doc](doc.md) for details.");

    let target = temp_dir.path().join("archive").join("doc.md");
    let output = run_cli(&[
        "mv",
        source.to_str().unwrap(),
        target.to_str().unwrap(),
        "--root",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());

    // Verify reference was updated
    let ref_content = read_file(&ref_file);
    assert!(ref_content.contains("archive/doc.md"));
    assert!(!ref_content.contains("](doc.md)"));
}

// ============= End-to-end: rename with reference update =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_updates_references_e2e() {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join("old_doc.md");
    let ref_file = temp_dir.path().join("index.md");
    write_file(&source, "# Old Document");
    write_file(&ref_file, "See [doc](old_doc.md) for info.");

    let output = run_cli(&[
        "rename",
        source.to_str().unwrap(),
        "new_doc.md",
        "--root",
        temp_dir.path().to_str().unwrap(),
    ]);

    assert!(output.status.success());

    let ref_content = read_file(&ref_file);
    assert!(ref_content.contains("new_doc.md"));
    assert!(!ref_content.contains("old_doc.md"));
}

// ============= dry-run CLI tests =============

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_mv_dry_run_does_not_move() {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join("source.md");
    let target = temp_dir.path().join("target.md");
    write_file(&source, "# Source");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Source](source.md)");

    let output = run_cli(&[
        "mv",
        source.to_str().unwrap(),
        target.to_str().unwrap(),
        "--root",
        temp_dir.path().to_str().unwrap(),
        "--dry-run",
    ]);

    assert!(output.status.success());

    // Source should still exist, target should not
    assert!(source.exists(), "Source should still exist after dry-run");
    assert!(
        !target.exists(),
        "Target should not be created during dry-run"
    );

    // Reference should be unchanged
    let ref_content = read_file(&ref_file);
    assert_eq!(ref_content, "[Source](source.md)");

    // Stdout should contain dry-run output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[dry-run]"),
        "Dry-run output should contain [dry-run] prefix. Got: {}",
        stdout
    );
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_cli_rename_dry_run_does_not_rename() {
    let temp_dir = temp_dir();
    let source = temp_dir.path().join("old.md");
    write_file(&source, "# Old name");

    let ref_file = temp_dir.path().join("ref.md");
    write_file(&ref_file, "[Old](old.md)");

    let output = run_cli(&[
        "rename",
        source.to_str().unwrap(),
        "new.md",
        "--root",
        temp_dir.path().to_str().unwrap(),
        "--dry-run",
    ]);

    assert!(output.status.success());

    // Source should still exist, new name should not
    assert!(source.exists(), "Source should still exist after dry-run");
    assert!(
        !temp_dir.path().join("new.md").exists(),
        "Renamed file should not be created during dry-run"
    );

    // Reference should be unchanged
    let ref_content = read_file(&ref_file);
    assert_eq!(ref_content, "[Old](old.md)");

    // Stdout should contain dry-run output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[dry-run]"),
        "Dry-run output should contain [dry-run] prefix. Got: {}",
        stdout
    );
}
