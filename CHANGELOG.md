# Changelog

All notable changes to this project will be documented in this file.

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
