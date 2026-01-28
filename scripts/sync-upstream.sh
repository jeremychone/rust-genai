#!/bin/bash
# Sync eckermann branch with upstream genai release
set -e

NEW_VERSION=$1

if [ -z "$NEW_VERSION" ]; then
  echo "Usage: ./sync-upstream.sh v0.5.2"
  echo ""
  echo "Available upstream tags:"
  git tag -l 'v*' | grep -v eckermann | tail -10
  exit 1
fi

echo "=== Syncing with upstream $NEW_VERSION ==="
echo ""

echo "Fetching upstream..."
git fetch upstream --tags

echo "Checking out eckermann branch..."
git checkout eckermann

echo ""
echo "Current patches on eckermann:"
git log --oneline $(git describe --tags --abbrev=0 upstream/main 2>/dev/null || echo "HEAD~5")..HEAD 2>/dev/null || echo "(no patches yet)"
echo ""

echo "Merging $NEW_VERSION..."
git merge $NEW_VERSION

echo ""
echo "======================================"
echo "SUCCESS! Next steps:"
echo ""
echo "1. Resolve conflicts if any"
echo "2. Run: git tag ${NEW_VERSION}-eckermann"
echo "3. Run: git push origin eckermann --tags"
echo "======================================"
