#!/usr/bin/env bash
set -e

cargo +nightly fmt
cargo check
cargo clippy --all-targets --all-features
cargo test
