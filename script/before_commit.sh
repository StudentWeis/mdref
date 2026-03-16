#!/usr/bin/env bash
set -e

# Check code formatting, linting, and tests before committing
./script/precheck.sh
./script/record_build_size.sh

# Run benchmarks to ensure the performance is not regressed before committing
cargo bench --bench benchmark
