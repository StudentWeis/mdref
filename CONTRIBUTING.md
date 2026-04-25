# Contributing to mdref

Thank you for considering contributing to mdref!

## Release

Releases are managed by `cargo-release`. Run `scripts/update_version.sh` to trigger the flow:

```bash
scripts/update_version.sh patch                   # dry-run a patch bump
scripts/update_version.sh 0.5.0 --execute         # release 0.5.0
```

The release pipeline (`cargo release`) automatically:
1. Bumps the version in `Cargo.toml` `[package]`.
2. Updates README installer links via `pre-release-replacements`.
3. Runs `scripts/release_prepare.sh` (pre-release hook) which executes `precheck.sh`, `record_build_size.sh`, a benchmark smoke test, generates the changelog with `git-cliff`, and verifies `dist plan`.
4. Commits, tags, and pushes — triggering the GitHub Actions release workflow.

## Code Style

- Follow [Clean Code](https://www.oreilly.com/library/view/clean-code-a/9780136083238/) principles
- Keep code simple (KISS) and avoid repetition (DRY)
- Use `thiserror` to define error types
- Before committing code, run the check script:

   ```bash
   ./scripts/precheck.sh
   ```

## Testing

See the [Testing Documentation](./doc/TESTING.md) for guidelines on writing and running tests.

## Benchmarks

```bash
cargo bench                 # full benchmark suite
./scripts/bench.sh quick    # quick smoke test
```

## Submitting Changes

mdref follows an **Issue → Branch → PR → Squash Merge** workflow, executed via the local `gh` CLI. The complete, copy-paste-ready SOP lives in the [`mdref-contribution-flow`](./.agents/skills/mdref-contribution-flow/SKILL.md) skill — it is the single source of truth for branch naming, commit conventions, the `scripts/precheck.sh` gate, and the PR self-check. Both human and AI contributors should follow it.

## Reporting Issues

If you find a bug or have a feature suggestion, please submit it on the [Issues](https://github.com/StudentWeis/mdref/issues) page using the matching template.

Thank you for your contribution!
