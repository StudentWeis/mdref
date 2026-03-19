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
cargo release
```
