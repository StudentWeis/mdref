#!/usr/bin/env bash
set -euo pipefail

# Check if a version argument is provided
if [ -z "$1" ]; then
	echo "Usage: $0 <new_version>"
	echo "Example: $0 0.1.2"
	exit 1
fi

NEW_VERSION=$1
CARGO_TOML="Cargo.toml"

# Check if Cargo.toml exists
if [ ! -f "$CARGO_TOML" ]; then
	echo "Error: $CARGO_TOML file not found"
	exit 1
fi

echo "Updating version to: $NEW_VERSION ..."

# Use perl for replacement because it is more reliable in handling multiline patterns and cross-platform (macOS/Linux) than sed
# Replace version under [package]
perl -i -0777 -pe "s/(\[package\]\n(?:.*\n)*?version\s*=\s*\").*?\"/\${1}$NEW_VERSION\"/m" "$CARGO_TOML"

# Update version in README.md installation links
README_MD="README.md"
if [ -f "$README_MD" ]; then
	echo "Updating version in $README_MD ..."
	perl -i -pe "s|releases/download/[0-9]+\.[0-9]+\.[0-9]+|releases/download/$NEW_VERSION|g" "$README_MD"
fi

# Update CHANGELOG.md using git cliff
git cliff --unreleased --tag $NEW_VERSION --prepend CHANGELOG.md

dist plan

# Confirm before running cargo release --execute
while true; do
	read -r -p "Run 'cargo release --execute'? [Y/n] " REPLY
	REPLY=${REPLY:-Y}
	case "$REPLY" in
	[Yy]*)
		echo "Running cargo release --execute..."
		cargo release --execute
		break
		;;
	[Nn]*)
		echo "Release aborted by user."
		exit 0
		;;
	*) echo "Please answer Y or n." ;;
	esac
done

echo "Update completed!"
