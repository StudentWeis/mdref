#![cfg(unix)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[allow(clippy::unwrap_used)]
fn bench_script_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("script/bench.sh")
}

#[allow(clippy::unwrap_used)]
fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).unwrap();

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
#[allow(clippy::unwrap_used)]
fn test_compare_uses_strict_baseline_mode() {
    let temp_dir = TempDir::new().unwrap();
    let fake_bin = temp_dir.path().join("bin");
    let fake_cargo = fake_bin.join("cargo");
    let args_file = temp_dir.path().join("cargo-args.txt");

    fs::create_dir_all(&fake_bin).unwrap();
    write_executable(
        &fake_cargo,
        r#"#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$@" > "$FAKE_CARGO_ARGS_FILE"
for arg in "$@"; do
  if [[ "$arg" == "--baseline-lenient" ]]; then
    echo "lenient baseline mode is not allowed for compare" >&2
    exit 91
  fi
done
"#,
    );

    let current_path = env::var("PATH").unwrap();
    let output = Command::new("bash")
        .arg(bench_script_path())
        .args(["compare", "main"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("FAKE_CARGO_ARGS_FILE", &args_file)
        .env("PATH", format!("{}:{current_path}", fake_bin.display()))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "bench script failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let args = fs::read_to_string(args_file).unwrap();
    assert!(
        args.contains("--baseline\nmain\n"),
        "unexpected args: {args}"
    );
    assert!(
        !args.contains("--baseline-lenient"),
        "unexpected args: {args}"
    );
}
