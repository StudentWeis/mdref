# Changelog

All notable changes to this project will be documented in this file.

## [0.4.4] - 2026-04-25

### 🚀 Features
- Add json output format for mv and rename commands by @StudentWeis
- Add JSON output format for find command and update dependencies by @StudentWeis
- Add progress bar functionality to file operations by @StudentWeis

### 💼 Other
- Optimize release profile settings for better binary size and performance by @StudentWeis

### 🚜 Refactor
- Improve error handling and consistency across modules by @StudentWeis

### ⚙️ Miscellaneous Tasks
- Add CI workflow, issue/PR templates, and contribution flow skill (#1) by @StudentWeis in [#1](https://github.com/StudentWeis/mdref/pull/1)
- Clean up changelog and update release script checks by @StudentWeis

## [0.4.3] - 2026-04-18

### 🚀 Features
- Add gitignore support for directory scanning by @StudentWeis

### 🚜 Refactor
- Enhance file moving logic with error handling by @StudentWeis
- Remove obsolete files and update dependencies; enhance pre-commit scripts and testing guidelines by @StudentWeis
- Deduplicate write_file test helper into shared test_utils module by @StudentWeis
- Remove pathdiff dependency and enhance url_decode_link function with multibyte UTF-8 support by @StudentWeis
- Remove pathdiff dependency and implement diff_paths function by @StudentWeis

### ⚙️ Miscellaneous Tasks
- Enhance release process with new scripts and hooks by @StudentWeis

## [0.4.2] - 2026-03-19

### 🚀 Features

- Enhance link reference handling in mv functionality by @StudentWeis
- Enhance testing framework by @StudentWeis
- Add rtk rewrite hook and enhance directory move tests for resource references by @StudentWeis
- Implement directory move functionality with reference updates and rollback support by @StudentWeis
- Implement link reference definition parsing and update by @StudentWeis
- Enhance link resolution by adding URL decoding for paths and improve error handling by @StudentWeis

### 🚜 Refactor

- Deduplicate strip_utf8_bom_prefix function by @StudentWeis
- Update dependencies and improve code organization across multiple files by @StudentWeis
- Update benchmark metrics to include directory move rewrites and enhance fixture tests by @StudentWeis
- Streamline move operation handling in benchmarks and tests by @StudentWeis
- Enhance error handling for UTF-8 input in find_links and find_references by @StudentWeis
- Enhance test fixtures for improved readability and maintainability by @StudentWeis
- Standardize test naming conventions and improve test descriptions by @StudentWeis
- Streamline error handling and enhance test coverage for mv functionality by @StudentWeis
- Enhance tests and command output handling by @StudentWeis
- Rename identifier for clarity by @StudentWeis

### ⚙️ Miscellaneous Tasks

- Format shell scripts for consistency and readability by @StudentWeis
- Add Unicode tests by @StudentWeis
- Standardize shebang and add error handling in script files by @StudentWeis
- Add confirmation prompt before executing cargo release in update_version script by @StudentWeis
- Implement comprehensive benchmarking framework and tests by @StudentWeis

## [0.4.1] - 2026-03-16

### 🚀 Features
- Implement utility functions for link processing and update tests to filter external URLs by @StudentWeis
- Refactor model module to include LinkReplacement and MoveTransaction, and update mv.rs to use these new structures by @StudentWeis
- Implement transaction support for mv_file to ensure atomicity and rollback on failure by @StudentWeis
- Enhance mv_file to support moving files into existing directories and update references accordingly by @StudentWeis
- Add checks to prevent overwriting existing files during move operations by @StudentWeis
- Add rename to lib by @StudentWeis
- Add dry-run option for mv and rename commands to preview changes without modifying files by @StudentWeis
- Implement link anchor stripping and preserve anchors during file moves by @StudentWeis

### 🐛 Bug Fixes
- Preserve pure anchor links during file moves by @StudentWeis
- Enhance mv_file to check for source existence and handle path normalization for identical files by @StudentWeis
- Preserve anchors in internal links during file moves and handle broken links gracefully by @StudentWeis
- Ensure precise link replacement in Markdown files to avoid incorrect updates for identical links on the same line by @StudentWeis
- Preserve external URLs during file move operation by @StudentWeis

### 🚜 Refactor
- Replace custom test directory setup with tempfile for improved test isolation by @StudentWeis
- Enhance link replacement logic to ensure precise updates in Markdown files by @StudentWeis
- Improve test setup using tempfile crate by @StudentWeis

### 🧪 Testing
- Add comprehensive testing framework and improve existing tests for better coverage and adherence to TDD principles by @StudentWeis

### ⚙️ Miscellaneous Tasks
- Update dependencies to latest versions for improved stability and performance by @StudentWeis

## [0.4.0] - 2026-03-13

### 🚀 Features
- Add configuration files, update Rust toolchain, and enhance pre-check scripts by @StudentWeis

### 🚜 Refactor
- Improve find command with comrak by @StudentWeis

## [0.3.6] - 2025-10-13

### 💼 Other
- Update README for performance metrics, enhance find command output, and add comprehensive tests for link and reference functionalities by @StudentWeis

## [0.3.5] - 2025-10-11

### 💼 Other
- Bump version to 0.3.5, update dependencies, and refactor reference handling by @StudentWeis
- Refactor error handling in find command and update README checklist by @StudentWeis

## [0.3.4] - 2025-10-11

### 💼 Other
- Bump version to 0.3.4, add error handling, and refactor command functions by @StudentWeis

## [0.3.3] - 2025-10-01

### 💼 Other
- Bump version to 0.3.3 and update installation links in README by @StudentWeis
- Refactor the project by @StudentWeis
- Enhance documentation in find.md and improve comments in find, mv, and rename commands by @StudentWeis

## [0.3.2] - 2025-09-29

### 💼 Other
- Add launch configuration, enhance README, and update find and mv functionalities by @StudentWeis

## [0.3.1] - 2025-09-29

### 💼 Other
- Implement find_links function by @StudentWeis

## [0.3.0] - 2025-09-23

### 💼 Other
- Update version to 0.3.0 by @StudentWeis
- Refactor output formatting in find and rename commands; enhance References struct with Display implementation by @StudentWeis
- Fix typo in CONTRIBUTING file: change 'git publish' to 'cargo publish' by @StudentWeis

## [0.2.0] - 2025-09-23

### 💼 Other
- Update version to 0.2.0 by @StudentWeis
- Add the implementation of the mv command by @StudentWeis
- Add mv_references function for moving file references; update Cargo.toml and README by @StudentWeis

## [0.1.3] - 2025-09-23

### 💼 Other
- Bump version to 0.1.3 and update installation scripts in README by @StudentWeis
- Enhance find_references function to return structured References; update output format and add column information by @StudentWeis
- Simplify find_references function by directly returning collected references; add rust-toolchain configuration file for stable channel by @StudentWeis
- Refactor find_references function for improved link processing using OnceLock by @StudentWeis

## [0.1.2] - 2025-09-18

### 💼 Other
- Add rayon for parallel processing; update README and fix image reference by @StudentWeis
- Update CONTRIBUTING guidelines and adjust benchmark directory depth in mock_generator by @StudentWeis
- Enhance project structure: update dependencies, add benchmarks, and improve find_references function; remove unused files by @StudentWeis
- Add CONTRIBUTING guidelines and update README for clarity by @StudentWeis
- Refactor find_references function and improve link processing; update README and .gitignore for clarity by @StudentWeis
