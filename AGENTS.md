# Repository Guidelines

## Project Structure & Module Organization
The root crate lives in `src/` with modules like `adapter/`, `chat/`, `client/`, `common/`, `embed/`, `resolver/`, and `webc/`. Shared helpers stay in `support.rs` and `tests/support/`, while integration suites sit in `tests/`. Examples run from `examples/` (try `cargo run --example c00-readme`). Docs and migrations are in `doc/`. Touch `terraphim-llm-proxy/` only when the Rust API changes.

## Build, Test, and Development Commands
- `cargo fmt` — formats the workspace using the repo’s `rustfmt.toml` (tabs, 120-column width).
- `cargo clippy --all-targets --all-features -- -D warnings` — enforces lint cleanliness across library, examples, and tests.
- `cargo test` — runs fast unit and mock-backed integration coverage.
- `cargo test --test live_api_tests -- --ignored` — exercises real Anthropic, OpenRouter, and Together flows; requires the corresponding API keys in your environment.

## Coding Style & Naming Conventions
Formatting is enforced via `rustfmt` (tabs, 120 max width, 80 for chains/arrays). Follow Rust idioms: modules stay `snake_case`, types use `UpperCamelCase`, constants use `SCREAMING_SNAKE_CASE`. Prefer `Client` builders and `ChatRequest::new(...)` ergonomics over manual struct literals. Document new public items and mirror the existing module layout when expanding providers or resolvers.

## Testing Guidelines
Default tests use mocked fixtures in `tests/support/` and `tests/data/`, so they pass offline. Live suites (`tests/live_api_tests.rs`, `tests_p_*`) auto-skip when keys like `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`, or `TOGETHER_API_KEY` are absent. Gate expensive cases with `#[ignore]`, note the run command in the header, and call out new env vars in your PR.

## Rust Performance Practices
- Profile first (`cargo bench`, `cargo flamegraph`, `perf`) and land only measured wins.
- Borrow ripgrep tactics: reuse buffers with `with_capacity`, favor iterators, reach for `memchr`/SIMD, and hoist allocations out of loops.
- Apply inline directives sparingly—mark tiny wrappers `#[inline]`, keep cold errors `#[cold]`, and guard cleora-style `rayon::scope` loops with `#[inline(never)]`.
- Prefer zero-copy types (`&[u8]`, `bstr`) and parallelize CPU-bound graph work with `rayon`, feature-gated for graceful fallback.

## Commit & Pull Request Guidelines
- Use Conventional Commit prefixes (`fix:`, `feat:`, `refactor:`) and keep changes scoped.
- Ensure commits pass `cargo fmt`, `cargo clippy`, required `cargo test`, and desktop checks.
- PRs should explain motivation, link issues, list manual verification commands, and attach UI screenshots or logs when behavior shifts.

## Configuration & Security Tips
- Keep secrets in 1Password or `.env`.
- Use `build-env.sh` or `scripts/` helpers to bootstrap integrations and wrap optional features (`openrouter`, `mcp-rust-sdk`) with graceful fallbacks for network failures.
