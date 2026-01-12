# Lessons Learned: Rust GenAI Project

This document captures insights, gotchas, and best practices discovered during development work on the rust-genai project.

---

## Session: 2025-11-03
**Focus:** Fix PR #3 CI failures and merge AWS Bedrock integration

### Technical Discoveries

#### 1. AdapterKind Debug Format vs Display Format
**Lesson:** Rust's derive(Debug) macro uses the exact enum variant name, including case quirks.

**Problem:**
```rust
// AdapterKind enum defined as:
pub enum AdapterKind {
    // ...
    ZAi,  // Note: capital 'A', lowercase 'i'
}

// Debug format returns: "ZAi"
// Test expected: "Zai"
// Result: Test failure
```

**Solution:** Always verify the actual Debug output before writing test assertions:
```bash
# Quick verification
cargo test --test test_name -- --nocapture | grep "adapter_kind"
```

**Best Practice:**
- Use `format!("{:?}", adapter_kind)` in test assertions (not Display or assumptions)
- When adding new AdapterKind variants, add tests immediately to verify Debug output
- Document expected Debug format in adapter module comments

#### 2. Model Routing Priority Order
**Lesson:** Model routing in `adapter_kind.rs:from_model()` has strict priority order that affects test expectations.

**Routing Order (highest to lowest):**
1. **Namespace patterns** (e.g., `zai::glm-4.6`) → Explicit adapter
2. **Adapter MODELS list** → Adapter if model in list
3. **Prefix patterns** (e.g., `glm-*`) → Fallback adapter
4. **Default** → Ollama

**Gotcha:** A model like `glm-3-turbo` is NOT in ZAI's MODELS list, so it falls through to step 3 (prefix `glm-*`) → Zhipu adapter, NOT ZAi.

**Best Practice:**
- Always check the adapter's MODELS const before writing routing tests
- Read adapter comments for model support notes (e.g., "No turbo models supported")
- Test both: models in the list AND models that should route to fallback adapters

**File Reference:** `src/adapter/adapters/zai/adapter_impl.rs:16-20`
```rust
pub(in crate::adapter) const MODELS: &[&str] = &[
    "glm-4.6", "glm-4.5", "glm-4", "glm-4.1v", "glm-4.5v", "vidu", "vidu-q1",
    "vidu-2.0",
    // Note: No turbo models are supported by Z.AI  // <-- Important note!
];
```

#### 3. Let-Chain Syntax (Rust 2021 Edition)
**Lesson:** Let-chains eliminate nested if statements and improve readability.

**Before (collapsible if statement - clippy warning):**
```rust
if let Ok(content) = std::fs::read_to_string(path) {
    if let Some(data) = extract_model_array(&content, marker) {
        // process data
    }
}
```

**After (let-chain - clippy approved):**
```rust
if let Ok(content) = std::fs::read_to_string(path)
    && let Some(data) = extract_model_array(&content, marker)
{
    // process data
}
```

**Benefits:**
- Reduces nesting level
- Eliminates clippy warnings
- More readable for sequential error handling
- Works with any number of sequential let-conditions

**Clippy Warning:** `collapsible_if` → Use let-chains instead

**Best Practice:**
- Run clippy frequently during development
- Fix clippy warnings immediately (they pile up quickly)
- Let-chains work with `Result` and `Option` types

#### 4. AWS Bedrock Authentication: Bearer Token NOT SigV4
**Lesson:** Bedrock integration is simpler than expected - uses Bearer token auth instead of AWS Signature Version 4.

**Discovery Process:**
1. Initial Bedrock implementation used `aws-sigv4` crate (complex)
2. Commit `66c7594` removed SigV4 in favor of Bearer token
3. Much simpler implementation: `Authorization: Bearer <token>`

**Environment Variable:** `AWS_BEARER_TOKEN_BEDROCK`

**Best Practice:**
- Check existing adapter implementations before choosing auth method
- Bearer token is simpler and sufficient for Bedrock Converse API
- SigV4 adds complexity without benefit for this use case

**Files:**
- `src/adapter/adapters/bedrock/adapter_impl.rs` - Bearer token auth
- Removed: `src/adapter/adapters/bedrock/aws_auth.rs` - SigV4 implementation

#### 5. Squash Merge Strategy for Feature Branches
**Lesson:** Squash merging keeps commit history clean for cohesive features.

**Scenario:** Bedrock integration had 4 commits:
1. Initial adapter implementation
2. Fix: Switch from SigV4 to Bearer token
3. Test updates for Bearer token limitations
4. Documentation updates

**Problem:** These commits tell a story of iteration, not a clean feature.

**Solution:** Squash merge → single comprehensive commit
```bash
git merge origin/feature-branch --squash
git commit -m "feat: complete feature description

Squashes N commits from feature-branch:
- abc1234 initial commit
- def5678 second commit
- ghi9012 third commit"
```

**Benefits:**
- Clean git history on main
- Single revert point if needed
- Easier to understand what was added
- Preserves detailed history on feature branch

**When NOT to Squash:**
- Collaborative branches with multiple authors
- Long-running branches where commit history tells important story
- When each commit is independently valuable

#### 6. Git Safety Hooks
**Lesson:** Git hooks prevent accidental data loss - respect them!

**Encountered Hook:**
```bash
$ git branch -D fix/merge-upstream-ci
BLOCKED: git branch -D force-deletes without merge check. Use -d for safety.
```

**What Happened:**
- Branch `fix/merge-upstream-ci` had unmerged commits
- Hook blocked force delete to prevent data loss
- Required manual intervention outside Claude Code

**Best Practice:**
- Git hooks are there for a reason - don't bypass them lightly
- Use `git branch -d` (safe delete) not `-D` (force delete)
- If hook blocks, investigate WHY before forcing
- Keep backup branches for reference until you're certain

**Recovery:**
```bash
# Check what commits will be lost
git log fix/merge-upstream-ci --not main --oneline

# If truly safe, delete outside of protected environment
git branch -D fix/merge-upstream-ci
```

### Debugging Approaches That Worked

#### 1. Incremental Clippy Fixes
**Approach:** Fix clippy warnings file-by-file, test after each fix.

**Steps:**
```bash
# 1. Identify warnings
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep "error:"

# 2. Fix one file
# Edit file.rs

# 3. Verify fix
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | grep file.rs

# 4. Commit
git add file.rs && git commit -m "fix: resolve clippy in file.rs"
```

**Why It Works:**
- Small commits are easier to review
- Catch regressions immediately
- Can bisect if something breaks

#### 2. Test-Driven Model Routing Verification
**Approach:** Write tests first to understand routing, then fix expectations.

**Process:**
```bash
# 1. Run model resolution test to see actual behavior
cargo test test_model_resolution -- --nocapture

# 2. Compare actual vs expected
# Actual: glm-3-turbo -> Zhipu
# Expected: glm-3-turbo -> ZAi

# 3. Investigate WHY
# Check adapter MODELS list
grep "glm-3-turbo" src/adapter/adapters/zai/adapter_impl.rs  # Not found
grep "MODELS" src/adapter/adapters/zai/adapter_impl.rs       # See comment

# 4. Fix test expectations to match actual behavior
# (Or fix routing if test expectation is correct)
```

**Key Insight:** Tests document behavior. If test fails, either code OR test expectations might be wrong.

#### 3. Binary Search for Regressions
**Approach:** Use git bisect when something breaks.

```bash
# Start bisect
git bisect start
git bisect bad HEAD  # Current state is broken
git bisect good <known-good-commit>

# Test automatically
git bisect run cargo test

# When done, bisect identifies the breaking commit
```

**Not needed this session**, but good to know for future.

### Pitfalls to Avoid

#### 1. Don't Assume Debug Format
**Pitfall:** Writing test assertions based on assumptions about format strings.

```rust
// WRONG
assert_eq!(format!("{:?}", adapter_kind), "Zai");  // Assumes lowercase 'a'

// RIGHT
let actual = format!("{:?}", adapter_kind);
println!("Actual format: {}", actual);  // Verify first
assert_eq!(actual, "ZAi");  // Use actual output
```

#### 2. Don't Ignore Adapter Comments
**Pitfall:** Missing important model support notes in adapter code.

```rust
// Adapter code clearly states:
// Note: No turbo models are supported by Z.AI

// But test tries:
let zai_models = vec!["glm-4.6", "glm-4", "glm-3-turbo"];  // Wrong!
```

**Fix:** Read adapter comments before writing tests.

#### 3. Don't Batch Unrelated Changes
**Pitfall:** Combining multiple fixes in one commit makes debugging harder.

**Bad:**
```bash
git commit -m "fix: lots of stuff
- clippy warnings
- test expectations
- new feature
- formatting"
```

**Good:**
```bash
git commit -m "fix: resolve clippy warnings in test files"
git commit -m "test: fix model resolution expectations"
git commit -m "feat: add new adapter"
```

**Why:** Smaller commits = easier to review, revert, and understand.

#### 4. Don't Skip Pre-Merge Verification
**Pitfall:** Merging without running full test suite.

**Always Run Before Merge:**
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build
```

**Lesson Learned:** This session ran verification after each commit, caught issues immediately.

#### 5. Don't Delete Branches Without Verification
**Pitfall:** Deleting feature branches before confirming merge succeeded.

**Wrong:**
```bash
git merge feature-branch
git branch -d feature-branch  # Too soon!
git push  # Oops, push failed, branch gone
```

**Right:**
```bash
git merge feature-branch
git push  # Verify push succeeds first
git branch -d feature-branch  # Then delete
```

### Best Practices Discovered

#### 1. Model List Consistency Testing
**Practice:** Have tests that verify hardcoded model lists match adapter code.

**Implementation:**
- `tests/test_adapter_consistency.rs` - Parses adapter source files to extract MODELS arrays
- `tests/test_verify_model_lists.rs` - Tests model resolution against actual provider APIs

**Benefit:** Catches drift when adapters add/remove models but tests aren't updated.

#### 2. Namespace-Based Routing Tests
**Practice:** Test both namespaced and non-namespaced model names.

**Test Cases:**
```rust
// Namespaced (explicit routing)
("zai::glm-4.6", "ZAi"),
("openai::gpt-4o", "OpenAI"),

// Non-namespaced (pattern-based routing)
("glm-4.6", "ZAi"),  // In ZAI MODELS list
("glm-2", "Zhipu"),  // NOT in ZAI MODELS list, fallback
```

**Benefit:** Verifies routing logic works for both explicit and fallback cases.

#### 3. Live API Testing Strategy
**Practice:** Separate live API tests into their own test file, make them optional.

**Implementation:**
```rust
// Only run if API key is available
if std::env::var("ZAI_API_KEY").is_err() {
    println!("⚠️  ZAI_API_KEY not set, skipping integration test");
    return Ok(());
}
```

**Benefit:** CI doesn't fail due to missing API keys, developers can run locally.

#### 4. Comprehensive Commit Messages
**Practice:** Use conventional commit format with detailed explanations.

**Template:**
```
<type>: <subject>

<optional detailed explanation>

<optional technical details>

Co-Authored-By: <name> <email>
```

**Example:**
```
fix: resolve clippy warnings in test_adapter_consistency.rs

- Collapse sequential if statements using let-chains
- Fix case mismatch: ZAi (Debug output) vs Zai (test expectations)
- Update all Zai references to ZAi to match AdapterKind::Debug format
- All tests passing

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Benefit:** Git log becomes searchable and informative.

#### 5. Handover Documentation
**Practice:** Create comprehensive handover documents after major work sessions.

**What to Include:**
- Progress summary (what was done)
- Technical context (current state, files changed)
- Next steps (what's left to do)
- Lessons learned (this document!)
- Commands for verification

**Benefit:** Seamless context transfer between sessions/developers.

---

## Quick Reference

### Useful Commands

**Check Model Routing:**
```bash
cargo test --test test_verify_model_lists test_provider_model_lists -- --nocapture
```

**Run All Adapter Tests:**
```bash
cargo test --test test_adapter_consistency \
            --test test_verify_model_lists \
            --test test_zai_adapter \
            --test tests_p_bedrock
```

**Verify CI Checks:**
```bash
cargo fmt --check && \
cargo clippy --all-targets --all-features -- -D warnings && \
cargo build && \
cargo test
```

**Check Branch Status:**
```bash
git status -sb  # Shows branch tracking status
git log origin/main..HEAD  # Shows unpushed commits
```

### File Locations

**Adapter Implementations:** `src/adapter/adapters/<provider>/adapter_impl.rs`
**Model Lists:** Look for `pub(in crate::adapter) const MODELS: &[&str] = &[...]`
**Routing Logic:** `src/adapter/adapter_kind.rs` - `from_model()` function
**Test Consistency:** `tests/test_adapter_consistency.rs`
**Model Resolution:** `tests/test_verify_model_lists.rs`

### Key Patterns

**Let-Chain Syntax:**
```rust
if let Ok(x) = result_1
    && let Some(y) = result_2
    && let Ok(z) = result_3
{
    // all passed
}
```

**AdapterKind Display/Debug:**
```rust
let adapter_kind = AdapterKind::ZAi;
println!("{:?}", adapter_kind);  // Prints: ZAi
```

**Model Resolution:**
```rust
let client = Client::default();
let target = client.resolve_service_target("glm-4.6").await?;
println!("{:?}", target.model.adapter_kind);  // Prints: ZAi
```

---

## End of Lessons Learned

Last Updated: 2025-11-03
Session: Fix PR #3 and merge Bedrock integration
Status: All tasks completed successfully
