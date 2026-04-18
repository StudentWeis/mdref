#!/usr/bin/env bash
set -euo pipefail

# Determine which command to use for Rust operations
if command -v rtk &>/dev/null; then
	CARGO_CMD="rtk cargo"
else
	CARGO_CMD="cargo"
fi

$CARGO_CMD +nightly fmt
$CARGO_CMD check
$CARGO_CMD clippy --all-targets --all-features
$CARGO_CMD test
$CARGO_CMD test --benches --no-run

# Format shell scripts with shfmt if it's available
if command -v shfmt &>/dev/null; then
	shfmt -w **/*.sh
fi
