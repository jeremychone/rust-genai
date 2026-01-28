# Fork Maintenance Guide

This document describes how to maintain our fork of [jeremychone/rust-genai](https://github.com/jeremychone/rust-genai) with custom OAuth patches while staying in sync with upstream releases.

## Repository Setup

### Remotes

| Remote | URL | Purpose |
|--------|-----|---------|
| `origin` | `https://github.com/holovskyi/rust-genai` | Our fork |
| `upstream` | `https://github.com/jeremychone/rust-genai.git` | Original repository |

### Initial Setup

```bash
# Clone your fork
git clone https://github.com/holovskyi/rust-genai.git
cd rust-genai

# Add upstream remote
git remote add upstream https://github.com/jeremychone/rust-genai.git
git fetch upstream --tags

# Verify remotes
git remote -v
```

## Branch Strategy

```
upstream/main (official releases)
       |
       +-- main (synced with upstream, for PRs)
       |     |
       |     +-- fix/* (bug fix branches for upstream PRs)
       |
       +-- eckermann (OAuth + custom patches)
```

| Branch | Purpose | Push to upstream? |
|--------|---------|-------------------|
| `main` | Synced with upstream, base for PRs | No |
| `eckermann` | Main working branch with OAuth support | No |
| `fix/*` | Bug fix branches for upstream PRs | Yes (via PR) |

## Commit Categories

We maintain two types of changes:

### 1. OAuth/Custom Features (Private)

These stay only in `eckermann` branch:
- OAuth token support (`sk-ant-oat-*`)
- Token refresh functionality
- OAuth request/response transformations
- Custom examples (c13, c14, c15)

**Commit prefix:** `feat(oauth):`, `fix(oauth):`, `^ oauth -`

### 2. Bug Fixes (Contribute to Upstream)

Generic fixes that benefit everyone:
- Bugs in existing Anthropic adapter
- Streaming issues
- Caching problems

**Commit prefix:** `fix(anthropic):`, `fix(streaming):`

## Syncing with Upstream Releases

When a new version (e.g., `v0.5.2`) is released upstream:

### Step 1: Fetch and Check

```bash
git fetch upstream --tags

# See what's new
git log --oneline v0.5.1..v0.5.2

# Check for potential conflicts
git diff --stat v0.5.1..v0.5.2
```

### Step 2: Merge (Recommended Strategy)

We use **merge** (not rebase) because:
- Preserves commit history
- Safer for shared branches
- Easier conflict resolution
- Can be reverted if needed

```bash
git checkout eckermann
git merge v0.5.2 -m "merge: update to upstream v0.5.2"

# Resolve conflicts if any
# Then verify
cargo check
cargo test --lib
```

### Step 3: Push

```bash
git push origin eckermann
```

## Contributing Bug Fixes to Upstream

When you fix a bug that should go to the official repository:

### Step 1: Sync main with upstream

```bash
git fetch upstream
git checkout main
git merge upstream/main
git push origin main
```

### Step 2: Create fix branch

```bash
git checkout -b fix/descriptive-name
```

### Step 3: Cherry-pick bug fix commits

```bash
# Find your bug fix commits in eckermann
git log --oneline eckermann | grep "fix(anthropic)"

# Cherry-pick them
git cherry-pick <commit-hash>
```

### Step 4: Verify

```bash
# Check no OAuth dependencies leaked in
git diff main..fix/descriptive-name | grep -i oauth

# Verify compilation
cargo check
cargo test --lib
```

### Step 5: Create PR

```bash
git push origin fix/descriptive-name

# Create PR via gh CLI
gh pr create --repo jeremychone/rust-genai \
  --base main \
  --head holovskyi:fix/descriptive-name \
  --title "fix(anthropic): description" \
  --body "Description of the fix"
```

### Step 6: After PR is merged

```bash
# Update main
git fetch upstream
git checkout main
git merge upstream/main
git push origin main

# Delete fix branch
git branch -d fix/descriptive-name
git push origin --delete fix/descriptive-name

# Your eckermann already has these changes, no action needed
```

## Using in Projects

### Via Branch (recommended)

```toml
[dependencies]
genai = { git = "https://github.com/holovskyi/rust-genai", branch = "eckermann" }
```

### Via Local Path (for development)

```toml
[dependencies]
genai = { path = "../rust-genai" }
```

## Quick Reference

| Task | Command |
|------|---------|
| Fetch upstream | `git fetch upstream --tags` |
| List upstream tags | `git tag -l 'v*'` |
| Update eckermann to v0.5.2 | `git checkout eckermann && git merge v0.5.2` |
| Sync main with upstream | `git checkout main && git merge upstream/main` |
| Check OAuth code isolation | `git diff main..fix/xxx \| grep -i oauth` |
| Test cherry-pick | `git cherry-pick --no-commit <hash> && git reset --hard` |

## Troubleshooting

### Conflict During Merge

```bash
# See conflicting files
git status

# After resolving conflicts
git add <resolved-files>
git commit

# To abort if needed
git merge --abort
```

### Cherry-pick Conflicts

```bash
# If cherry-pick fails
git cherry-pick --abort

# Manual approach: create patch and apply
git show <commit> > fix.patch
git apply --3way fix.patch
```

### Checking What Patches We Have

```bash
# See all our commits on top of upstream tag
git log --oneline v0.5.2..eckermann

# See only OAuth-related
git log --oneline v0.5.2..eckermann | grep -i oauth

# See only bug fixes
git log --oneline v0.5.2..eckermann | grep "fix(anthropic)"
```

## Example: Full Update Cycle

```bash
# 1. New upstream release v0.5.3 is out
git fetch upstream --tags

# 2. Update eckermann
git checkout eckermann
git merge v0.5.3 -m "merge: update to upstream v0.5.3"
cargo check
git push origin eckermann

# 3. If we have bug fixes to contribute
git checkout main
git merge upstream/main
git push origin main

git checkout -b fix/some-bug
git cherry-pick abc123
cargo check
git push origin fix/some-bug
gh pr create --repo jeremychone/rust-genai ...
```
