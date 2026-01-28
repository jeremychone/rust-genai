# Fork Maintenance Guide

This document describes how to maintain our fork of [jeremychone/rust-genai](https://github.com/jeremychone/rust-genai) with custom patches while staying in sync with upstream releases.

## Repository Setup

### Remotes

| Remote | URL | Purpose |
|--------|-----|---------|
| `origin` | `git@github.com:YOUR_USERNAME/rust-genai.git` | Your fork |
| `upstream` | `https://github.com/jeremychone/rust-genai.git` | Original repository |

### Initial Setup

```bash
# Clone your fork
git clone git@github.com:YOUR_USERNAME/rust-genai.git
cd rust-genai

# Add upstream remote
git remote add upstream https://github.com/jeremychone/rust-genai.git
git fetch upstream --tags

# Verify remotes
git remote -v
```

## Branch Strategy

| Branch | Purpose |
|--------|---------|
| `eckermann` | Main working branch with custom patches |
| `upstream/main` | Tracks original repository |

### Creating the Working Branch

```bash
# Create branch from a specific upstream release tag
git checkout -b eckermann v0.5.0
```

## Adding Custom Patches

```bash
# Work in the eckermann branch
git checkout eckermann

# Make changes, commit with clear messages
git add .
git commit -m "feat: custom memory slot support"

# Tag your custom version
git tag v0.5.0-eckermann
git push origin eckermann --tags
```

### Commit Message Guidelines

Keep patches atomic with clear descriptions to simplify future rebases:

```
feat: add memory slot to ChatRequest
fix: handle empty stream gracefully
feat: expose internal adapter state
```

## Syncing with Upstream Releases

When a new version (e.g., `v0.5.2`) is released upstream:

### Step 1: Fetch Updates

```bash
git fetch upstream --tags

# See what's new
git log v0.5.0..v0.5.2 --oneline
```

### Step 2: Integrate Changes

**Option A: Rebase (recommended for cleaner history)**

```bash
git checkout eckermann
git rebase v0.5.2

# Resolve conflicts if any
# Then force push (required after rebase)
git push origin eckermann --force-with-lease
```

**Option B: Merge (simpler, preserves history)**

```bash
git checkout eckermann
git merge v0.5.2

# Resolve conflicts if any
git push origin eckermann
```

### Step 3: Tag and Push

```bash
# Create new custom version tag
git tag v0.5.2-eckermann
git push origin eckermann --tags
```

## Using in Projects

### Via Git Tag (recommended for production)

```toml
[dependencies]
genai = { git = "https://github.com/YOUR_USERNAME/rust-genai", tag = "v0.5.2-eckermann" }
```

### Via Branch (for development)

```toml
[dependencies]
genai = { git = "https://github.com/YOUR_USERNAME/rust-genai", branch = "eckermann" }
```

### Via Local Path (for active development)

```toml
[dependencies]
genai = { path = "../rust-genai" }
```

## Automation Script

Save as `scripts/sync-upstream.sh`:

```bash
#!/bin/bash
set -e

NEW_VERSION=$1

if [ -z "$NEW_VERSION" ]; then
  echo "Usage: ./sync-upstream.sh v0.5.2"
  echo ""
  echo "Available upstream tags:"
  git tag -l 'v*' | grep -v eckermann | tail -10
  exit 1
fi

echo "Fetching upstream..."
git fetch upstream --tags

echo "Checking out eckermann branch..."
git checkout eckermann

echo "Merging $NEW_VERSION..."
git merge $NEW_VERSION

echo ""
echo "======================================"
echo "Resolve conflicts if any, then run:"
echo "  git tag ${NEW_VERSION}-eckermann"
echo "  git push origin eckermann --tags"
echo "======================================"
```

Make executable:

```bash
chmod +x scripts/sync-upstream.sh
```

## Quick Reference

| Task | Command |
|------|---------|
| Fetch upstream | `git fetch upstream --tags` |
| List upstream tags | `git tag -l 'v*' \| grep -v eckermann` |
| Update to v0.5.2 | `git merge v0.5.2` or `git rebase v0.5.2` |
| Tag custom version | `git tag v0.5.2-eckermann` |
| Push changes | `git push origin eckermann --tags` |
| Force push (after rebase) | `git push origin eckermann --tags --force-with-lease` |

## Troubleshooting

### Conflict Resolution During Rebase

```bash
# See conflicting files
git status

# After resolving conflicts
git add <resolved-files>
git rebase --continue

# To abort if needed
git rebase --abort
```

### Reverting to Clean State

```bash
# Reset to upstream tag (loses local changes!)
git checkout eckermann
git reset --hard v0.5.2
```

### Checking Patch Differences

```bash
# See what patches we have on top of upstream
git log v0.5.2..eckermann --oneline
```
