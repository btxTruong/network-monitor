#!/bin/bash
# Release script - updates Cargo.toml version, commits, and creates tag
# Usage: ./release.sh v0.1.0
# After running, push manually via IDE or: git push origin main && git push origin <tag>

set -e

VERSION="$1"

if [ -z "$VERSION" ]; then
    echo "Usage: ./release.sh vX.Y.Z"
    echo "Example: ./release.sh v0.2.0"
    exit 1
fi

# Validate version format
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in format vX.Y.Z (e.g., v0.1.0)"
    exit 1
fi

# Strip 'v' prefix for Cargo.toml
CARGO_VERSION="${VERSION#v}"

echo "Creating release $VERSION..."

# Update version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$CARGO_VERSION\"/" Cargo.toml
echo "✓ Updated Cargo.toml to version $CARGO_VERSION"

# Update Cargo.lock
cargo check --quiet 2>/dev/null || true
echo "✓ Updated Cargo.lock"

# Commit version bump
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $VERSION"
echo "✓ Committed version bump"

# Create tag
git tag -a "$VERSION" -m "Release $VERSION"
echo "✓ Created tag $VERSION"

echo ""
echo "Now push manually:"
echo "  1. Push main branch (via IDE or: git push origin main)"
echo "  2. Push tag: git push origin $VERSION"
echo ""
echo "After push, check: https://github.com/btxTruong/network-monitor/actions"
