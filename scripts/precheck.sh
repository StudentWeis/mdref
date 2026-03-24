#!/usr/bin/env bash
set -e

rtk cargo +nightly fmt
rtk cargo check
rtk cargo clippy --all-targets --all-features
rtk cargo test
rtk cargo test --benches --no-run
