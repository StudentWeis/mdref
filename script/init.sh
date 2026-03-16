#!/usr/bin/env bash
set -e

# Install prek for pre-commit hooks
prek install

# Install cargo-release and cargo-dist for release management and distribution
cargo install cargo-release cargo-dist
