# Anthropic Platform Rust Test Plan

This plan translates the requirements into executable Rust test suites for the genai library. Each section maps the Anthropic Platform surface area to concrete `cargo test` targets, identifies fixtures/mocks, and calls out validation checkpoints (headers, payload schemas, streaming, and error handling).

## Core Messages API
- `POST /v1/messages` happy path (non-streaming): validate required headers (`x-api-key`, `anthropic-version`), response schema, usage token accounting.
- Streaming variant: drive `ChatRequest::stream = Some(true)` through the genai handler, assert SSE framing and final message assembly.
- Tool calling: include `tools` and `tool_choice`, confirm tool outputs are echoed.
- Multimodal messages: include `ContentBlock::Image` and ensure payload normalization.
- Error handling: simulate invalid model, missing parameters, and propagate `ProxyError`.

## Token Counting
- `POST /v1/messages/count_tokens` round-trip against Anthropic mock server verifying `input_tokens` field alignment with `TokenCounter`.
- Boundary cases: empty conversation, maximum context window (200k), cache-control system prompts.

## Message Batches
- Submission (`POST /v1/messages/batches`): validate batch envelope, custom IDs, and metadata propagation.
- Polling endpoints: `GET /v1/messages/batches/{batch_id}` and `/results` using paginated fixtures to ensure deserialization and continuation token handling.
- Cancellation and deletion flows: exercise `cancel` (with optional reason) and `DELETE` endpoints, assert status transitions `in_progress → canceled`.

## Files API
- Upload (`POST /v1/files`): multipart builder helper, ensure boundary formatting and metadata passthrough.
- Listing (`GET /v1/files`): pagination tests with `has_more` toggles.
- Metadata retrieval and download: confirm binary streaming and content-type preservation.
- Deletion: verify 204 response and idempotent behaviour.

## Models API
- Catalog (`GET /v1/models`): deserialize pricing/context metadata, compare against routing configuration expectations.
- Single model lookup (`GET /v1/models/{model_id}`): assert alias resolution and capability flags (tool support, context window).

## Experimental Prompt Tools
- `POST /v1/experimental/generate_prompt`: ensure optional beta headers are injected and response structures match spec.

## Cross-Cutting Scenarios
- Header contract: reusable assertion helper to check Anthropic diagnostic headers (`request-id`, `anthropic-organization-id`) on every response.
- Timeout/resiliency: simulate transient network failures with `RetryExecutor`, assert retry backoff and logging.
- Intelligent routing integration: run end-to-end tests where Anthropic is selected via markdown-driven routing and verify request transformation layers.

## Implementation Notes
- Use `#[cfg(feature = "anthropic-live")]` gated tests for real API calls; default suite relies on mocks.
- Provide fixture builders in `tests/support/anthropic.rs` to keep test setup concise.
- Record golden JSON payloads in `tests/data/anthropic/` for snapshot comparisons.
- Update CI pipeline matrix to run `cargo test --features anthropic-live` nightly with sanitized secrets.

## Additional Test Areas for genai

### Reasoning Models
- Test Claude thinking models with reasoning budget
- Validate reasoning usage reporting
- Test reasoning effort parameters

### Caching
- Explicit cache control headers
- Implicit caching behavior
- Cache hit/miss validation

### Vision/Multimodal
- Image URL support
- Base64 image encoding
- PDF document processing
- Multi-modal message handling

### Rate Limiting
- Header-based rate limit detection
- Retry-after header handling
- Concurrent request limits