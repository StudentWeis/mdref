Thanks to contribute to the mdref project!

There are some things to note before you submit your PR.

1. AI-assisted code generation is permitted, but it must be manually reviewed by the contributor and indicated as "AI-assisted" in the PR.

# Test

```sh
./target/release/mdref rename ./examples/main.md test.md
./target/release/mdref rename ./examples/test.md main.md
```

# Bench

```sh
cargo bench
```

# Release

```sh
./scripts/update_version.sh patch
./scripts/update_version.sh 0.5.0 --execute
```

The wrapper delegates to `cargo release`. README installer links are updated via
`cargo release` replacements, and changelog generation plus release checks run in
the pre-release hook.
