#!/usr/bin/env bash
set -e

# Check code formatting, linting, and tests before committing
./script/precheck.sh
./script/record_build_size.sh

# Run a quick benchmark smoke check before committing
./script/bench.sh quick
