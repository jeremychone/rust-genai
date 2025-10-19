# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Essential Development Commands

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run live API tests (requires API keys)
cargo test --test live_api_tests -- --nocapture

# Run specific test file
cargo test --test live_api_tests
```

### Code Quality
```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt

# Run clippy with strict warnings
cargo clippy --all-targets --all-features -- -W clippy::all

# Run clippy with default settings
cargo clippy --all-targets --all-features
```

### Building
```bash
# Build the library
cargo build

# Build with release optimizations
cargo build --release
```

## Live API Testing Requirements

The project includes comprehensive live API tests that require real API keys:

**Required Environment Variables:**
- `OPENROUTER_API_KEY` - For OpenRouter tests
- `ANTHROPIC_API_KEY` - For Anthropic tests
- `CEREBRAS_API_KEY` - For Cerebras tests (optional)
- `ZAI_API_KEY` - For Z.AI tests (optional)

Live API tests are located in `/tests/live_api_tests.rs` and include:
- Basic chat functionality
- Streaming support
- Tool/function calling
- JSON mode
- Image processing
- Cross-provider compatibility

## Architecture Overview

### Core Structure
- **Client Layer** (`src/client/`): Main public API providing unified interface across AI providers
- **Adapter Layer** (`src/adapter/`): Provider-specific implementations using static dispatch pattern
- **Common Types** (`src/common/`): Shared data structures across the library
- **Chat Module** (`src/chat/`): Chat completion functionality and types
- **Embed Module** (`src/embed/`): Embedding support
- **Resolver Module** (`src/resolver/`): Model name to adapter resolution logic

### Adapter Pattern
The library uses an adapter pattern to normalize APIs across different AI providers:
- Each provider (OpenAI, Anthropic, Gemini, etc.) has its own adapter implementation in `src/adapter/adapters/`
- The `AdapterDispatcher` routes requests to appropriate adapters based on model naming conventions
- Default model-to-adapter mapping follows prefix rules (e.g., "gpt" → OpenAI, "claude" → Anthropic)

### Key Components
- **ServiceTarget**: Represents endpoint, authentication, and model configuration
- **AdapterKind**: Enum representing each AI provider type
- **ChatRequest/ChatResponse**: Core types for chat completions
- **MessageContent**: Multi-part content support (text, images, PDFs)
- **ChatOptions**: Configuration parameters (temperature, max_tokens, etc.)

## Provider Support

Currently supports these AI providers:
- OpenAI (including gpt-5-codex via Responses API)
- Anthropic
- Gemini (native protocol support)
- OpenRouter
- Groq
- xAI/Grok
- Ollama
- DeepSeek (including DeepSeekR1 reasoning_content)
- Cohere
- Cerebras
- Z.AI (GLM models, OpenAI-compatible API)
- And more...

## Testing Guidelines

- Keep fast unit tests inline with `mod tests {}`; put multi-crate checks in `tests/` or `test_*.sh`
- Scope runs with `cargo test -p crate test`; add regression coverage for new failure modes
- Live API tests require real API keys and are located in `/tests/live_api_tests.rs`

## Rust Performance Practices

- Profile first (`cargo bench`, `cargo flamegraph`, `perf`) and land only measured wins
- Borrow ripgrep tactics: reuse buffers with `with_capacity`, favor iterators, reach for `memchr`/SIMD, and hoist allocations out of loops
- Apply inline directives sparingly—mark tiny wrappers `#[inline]`, keep cold errors `#[cold]`, and guard cleora-style `rayon::scope` loops with `#[inline(never)]`
- Prefer zero-copy types (`&[u8]`, `bstr`) and parallelize CPU-bound graph work with `rayon`, feature-gated for graceful fallback

## Commit & Pull Request Guidelines

- Use Conventional Commit prefixes (`fix:`, `feat:`, `refactor:`) and keep changes scoped
- Ensure commits pass `cargo fmt`, `cargo clippy`, required `cargo test`, and desktop checks
- PRs should explain motivation, link issues, list manual verification commands, and attach UI screenshots or logs when behavior shifts

## Configuration & Security Tips

Keep secrets in 1Password or `.env`. Use `build-env.sh` or `scripts/` helpers to bootstrap integrations, and wrap optional features (`openrouter`, `mcp-rust-sdk`) with graceful fallbacks for network failures.

## Development Guidelines

- No unsafe code allowed (forbidden in Cargo.toml)
- Use async/await throughout (tokio runtime)
- Follow Rust 2024 edition conventions
- Add comprehensive error handling using the library's Result type
- Include tracing for debugging
- Write tests for new functionality
- Document public APIs
- Never commit API keys - use environment variables only