#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Check code formatting, linting, and tests before release
./scripts/precheck.sh
./scripts/record_build_size.sh

# Run a quick benchmark smoke check before release
./scripts/bench.sh quick
