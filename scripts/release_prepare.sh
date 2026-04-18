#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

read_package_version() {
	perl -0777 -ne 'print "$1\n" and exit if /\[package\]\n(?:.*\n)*?version\s*=\s*"([^"]+)"/m' Cargo.toml
}

if [[ "${DRY_RUN:-false}" == "true" ]]; then
	echo "Skipping release preparation during cargo release dry-run."
	exit 0
fi

release_version="${NEW_VERSION:-$(read_package_version)}"

if [[ -z "$release_version" ]]; then
	echo "Error: failed to resolve release version from Cargo.toml" >&2
	exit 1
fi

bash scripts/before_update.sh
git cliff --unreleased --tag "$release_version" --prepend CHANGELOG.md
dist plan
