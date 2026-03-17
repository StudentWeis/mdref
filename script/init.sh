#!/usr/bin/env bash
set -e

# For pre-commit hooks
prek install

# For release management and distribution
cargo install cargo-release cargo-dist
