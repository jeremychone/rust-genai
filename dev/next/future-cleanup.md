# Potential Cleanup to do

> Note: Those are just internal bookkeeping notes. They will eventually be done if needed, and should not be picked up unless they are part of another fix or enhancement.

## 2026-10-14 - PR 332 related - openai_resp done arguments

- Refresh `call_id` and `name` from `output_item.done` when the entry was created from an `output_item.added` that carried empty values.

- Emit a final `ToolCallChunk` when `output_item.done` overrides the accumulated arguments, so streaming consumers see the complete arguments.

- Finalize `in_progress_tool_calls` on the `ResponseIncomplete` and EOF paths. Today `captured_tool_calls` only reads `captured_data.tool_calls`, which is populated in `ResponseCompleted`, so those accumulated calls are dropped.

- Convert `tests/openai_resp_done_arguments.rs` into a yakbak protocol-fixture cassette under `tests/data/yakbak/openai_resp/`, add a `test_yakbak_openai_resp_done_arguments_override` case in `tests/tests_yakbak_openai_resp.rs`, and delete the raw TCP test.

- In the new test, use `TestResult<()>` with `?`, and check the length before indexing `calls[0]`, or use `.first().ok_or(...)`.

- In `tests/tests_yakbak_openai_resp.rs`, replace the byte-index string slices (`&reasoning[..reasoning.len().min(60)]` and the `content` tail slice) with char-boundary-safe truncation, since the content can be multi-byte (Japanese).
  - In `Cargo.toml`, extend the `native-tls` comment to mention re-adding `system-proxy`.

## 2026-10-15 - PR 334 related - v0.7.0 migration guide note

- Context: PR 334 changes behavior for `default-features = false` builds. They no longer read macOS or Windows OS proxy settings unless `system-proxy` is enabled.

- Why: the `CHANGELOG.md` entry is already added under API Breaking Changes. The v0.7.0 migration guide is maintainer-owned and was not updated.

- Suggested change: add a short migration note to the v0.7.0 migration guide that shows `features = ["system-proxy"]` as the way to keep the old behavior. Keep the wording aligned with the README note.

## 2026-10-15 - PR 334 related - proxy setters overwrite each other

- Context: in `src/client/web_config.rs`, `with_proxy_url`, `with_https_proxy_url`, and `with_all_proxy_url` each overwrite the same stored proxy, so only one proxy is kept.

- Why: this is not introduced by PR 334, but it can surprise callers who set an HTTP proxy and then an HTTPS proxy and expect both to apply.

- Suggested change: decide the intended semantics first. Either document that the last setter wins, or store the proxies separately per scheme and apply them in `apply_to_builder`. Add a unit test for the chosen behavior.

## 2026-10-15 - PR 335 related - confirm Vertex live tool tests

- Context: `VertexAdapter::to_gemini_web_request_data` calls `GeminiAdapter::build_gemini_request_payload`, so PR 335 also sends `functionCall.id` and `functionResponse.id` to Vertex.

- Why: the PR author ran only the Gemini live tests. Vertex defines the same `id` fields and is expected to accept them, but this is not verified.

- Suggested change: run the live Vertex tool tests in `tests/tests_p_vertex.rs`, especially `test_tool_gemini_full_flow_ok` and `test_tool_gemini_simple_ok`, before or just after merging. If one fails with 400, apply the same name and id rule that the Gemini path uses.

## 2026-10-15 - PR 335 related - infallible gemini part helpers

- Context: `gemini_function_call_part` and `gemini_function_response_part` return `Result<Value>` only because `x_insert` is fallible.

- Why: the reviewer noted this is acceptable and matches the file's style. Building the object as a `serde_json::Map` would make the helpers infallible and simpler to call.

- Suggested change: optional. Build the `functionCall` and `functionResponse` objects as `serde_json::Map` values, then return `Value` directly, if this makes the callers simpler. Keep it only if it reads clearly.

## 2026-10-15 - PR 335 related - align gemini_ix id policy

- Context: `gemini_ix` in `into_ix_input_steps` sends `call_id` as it is. This includes empty strings and synthetic `call#{name}#{n}` ids, which the `gemini` adapter now filters with `gemini_call_id`.

- Why: this is pre-existing and does not affect matching within `gemini_ix`. It only matters when histories from the `gemini` adapter are replayed through `gemini_ix`, and then the two adapters follow different id rules.

- Suggested change: optionally apply the same empty and synthetic id filter in `gemini_ix`, or share the helper. Check the Interactions API tests before changing it, since the current behavior is accepted by the existing tests.

## 2026-10-15 - PR 336 related - pastey adapter identifiers

- Context: `src/adapter/macros/dispatcher_macros.rs` uses `paste!` only to concatenate fixed tokens such as `[<OpenAI Adapter>]`.

- Why: the concatenation is cosmetic, because the tokens are fixed and known. Writing the identifiers directly would remove the direct `pastey` dependency from `genai`. `pastey` would still be pulled in transitively by `macro_rules_attribute`, so this is a cleanup, not a dependency fix.

- Suggested change: replace the `[<X Adapter>]` forms with literal identifiers such as `OpenAIAdapter`, then remove the direct `pastey` dependency from `Cargo.toml` and the `paste` import. Confirm the dispatch match still expands the same way.

## 2026-10-15 - PR 336 related - pre-merge dependency checks

- Context: PR 336 replaces `paste` with `pastey` and states that `cargo build`, `cargo clippy --all-targets`, `cargo test --lib`, and the yakbak replay tests pass.

- Why: a few checks were not reported. The `bedrock-sigv4` cfg arm of `dispatch_adapter!` was not explicitly built, `Cargo.lock` consistency was not mentioned, and `paste` leaving the tree was not confirmed.

- Suggested change: run `cargo tree -i paste` (expect nothing), `cargo build --features bedrock-sigv4`, and confirm `Cargo.lock` is updated if it is tracked. Record the results in the PR.


## ■ Step - Fix: finalize OpenAI Responses tool calls on incomplete and EOF
      status: not_started
time-created: 2026-10-15 15:51:23

Behavior change, needs yakbak coverage. Commit prefix: `- openai_resp - Finalize tool calls on incomplete and EOF`

- Refresh `call_id` and `name` from `output_item.done` when the entry came from an `output_item.added` with empty values.
- Emit a final `ToolCallChunk` when `output_item.done` overrides the accumulated arguments.
- Finalize `in_progress_tool_calls` on the `ResponseIncomplete` and EOF (`None`) paths so accumulated calls are not dropped.
- Add a yakbak cassette test and remove the raw TCP test. Blocked until the test files are provided.

## ■ Step - Refactor: literal adapter identifiers in dispatcher macros
      status: not_started
time-created: 2026-10-15 15:51:23

High-risk macro refactor. Commit prefix: `> macros - Use literal adapter identifiers`

- `src/adapter/macros/dispatcher_macros.rs`: replace `[<X Adapter>]` forms with literal identifiers such as `OpenAIAdapter`, and remove the `paste` import.
- `Cargo.toml`: remove the direct `pastey` dependency if nothing else uses it.
- Verify with `cargo tree -i pastey`, `cargo build`, and `cargo build --features bedrock-sigv4`.
