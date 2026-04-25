#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/precheck.sh           # local mode: auto-fix formatting, then check
#   scripts/precheck.sh --check   # CI mode: verify only, fail on any drift

CHECK_ONLY=0
if [[ "${1:-}" == "--check" ]]; then
	CHECK_ONLY=1
fi

# Determine which command to use for Rust operations
if command -v rtk &>/dev/null; then
	CARGO_CMD="rtk cargo"
else
	CARGO_CMD="cargo"
fi

if [[ $CHECK_ONLY -eq 0 ]]; then
	$CARGO_CMD +nightly fmt
	if command -v shfmt &>/dev/null; then
		shfmt -w **/*.sh
	fi
fi

$CARGO_CMD check
$CARGO_CMD clippy --all-targets --all-features
$CARGO_CMD test
$CARGO_CMD test --benches --no-run
