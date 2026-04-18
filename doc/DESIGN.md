# mdref design document

mdref is a command-line tool for managing references of Markdown files.

This document outlines the design and architecture of mdref.

## Functionality Design

Basic operations include:

- find: Find all references to a Markdown file.
- move: Move a Markdown file and update all references.

Other potential features:

- validate: Validate that all references in Markdown files are valid.
- report: Generate a report of all Markdown files and their references.

Other features:

- gitignore support: Ignore files/directories based on .gitignore patterns.
- atomic operations: Ensure move operations are atomic to prevent broken references.
