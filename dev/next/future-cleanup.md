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

## 2026-10-19 - PR 339 related - openai_resp terminal paths still drop captured state

- Context: PR 339 (`openai_resp - keep a stream's tool calls when the response is incomplete`) moved the tool-call capture out of `RespStreamEvent::ResponseCompleted` into `OpenAIRespStreamer::capture_tool_calls(&mut self, output: &[Value])` in `src/adapter/adapters/openai_resp/streamer.rs`, and now calls it from both `ResponseCompleted` and `ResponseIncomplete`.

- Resolved by PR 339: `response.incomplete` no longer loses the calls accumulated in `in_progress_tool_calls`. That closes the `ResponseIncomplete` half of the third bullet in `## 2026-10-14 - PR 332 related - openai_resp done arguments`. That bullet is kept verbatim for history, and only its EOF half is still open.

- Potential bug 1, EOF path: the `Poll::Ready(None)` branch (the `None => { if !self.done { ... } }` arm of `poll_next`) builds its `InterStreamEnd` from `captured_data` only and never calls `capture_tool_calls`, so `in_progress_tool_calls`, which is filled by `response.function_call_arguments.delta` and `response.custom_tool_call_input.delta`, is dropped.
  - It also leaves `captured_stop_reason` as `None`, because `captured_data.stop_reason` is only set by `response.completed`, `response.incomplete`, and `response.failed`.
  - This is the same failure mode PR 339 fixed, on the path taken when the event source ends without a terminal event: a dropped connection, a proxy closing the stream, or a backend that streams deltas only.
  - Suggested change: call `self.capture_tool_calls(&[])` before building the end in that branch (empty `output`, since there is no terminal response to fall back to), and add a test whose stream stops right after the argument deltas.

- Potential bug 2, `response.incomplete` drops reasoning signatures: `OutputItemDone` pushes a `type: "reasoning"` item's `encrypted_content` into `captured_data.thought_signatures`, and `ResponseCompleted` moves them into `InterStreamEnd::captured_thought_signatures`. The `ResponseIncomplete` branch hard-codes `captured_thought_signatures: None`, and it also skips the `response.output` scan that `ResponseCompleted` does, so blocks finalized before the cap are lost.
  - Impact: an incomplete answer keeps its text and its tool calls but not the continuation blocks OpenAI expects on the next turn, so replaying that assistant message is inconsistent with a completed answer.
  - Suggested change: `take()` `self.captured_data.thought_signatures` there as well, or fold the terminal paths into one helper that builds `InterStreamEnd` for completed, incomplete, and EOF.

- Potential bug 3, `response.incomplete` ignores `capture_usage`: `ResponseCompleted` stores usage only under `if self.options.capture_usage`, while `ResponseIncomplete` uses `response.usage.map(Into::into)` directly, so usage is reported on an incomplete answer even when the option is off.
  - Suggested change: gate it like the completed branch, ideally through the same shared helper.

- Test suite: `tests/openai_resp_incomplete_tool_calls.rs` spins up a raw `TcpListener`, the same pattern the 2026-10-14 item already wants converted. Fold it into that work: a yakbak cassette under `tests/data/yakbak/openai_resp/` plus a case in `tests/tests_yakbak_openai_resp.rs`, then delete the raw TCP server.

## 2026-10-19 - PR 340 related - anthropic max_tokens tool-call test and streamer follow-ups

- Context: PR 340 captures a `tool_use` block cut off by `max_tokens` at `message_stop`, through `AnthropicStreamer::capture_tool_call(id, name, input, cut_off)` in `src/adapter/adapters/anthropic/streamer.rs`, called from both `content_block_stop` and `message_stop`.

- Test file: `tests/anthropic_max_tokens_tool_calls.rs` spins up a raw `TcpListener`, and its name breaks the `tests/tests_*.rs` convention.
  - Suggested change: fold it into `tests/tests_yakbak_anthropic.rs` with a cassette under `tests/data/yakbak/anthropic/`, then delete the raw TCP server, the same conversion the 2026-10-14 item wants for the OpenAI Responses tests.

- Nit, helper placement: `capture_tool_call` sits in the top inherent `impl` block, while the file keeps its private helpers (`capture_usage`, `parse_message_data`) in the bottom `// Support` impl.
  - Suggested change: move it there, and consider replacing the `cut_off: bool` argument with two named wrappers or a small enum, since `.., false)` and `.., true)` do not read at the call sites.

- Missing test: no coverage for `capture_tool_calls == false`, which is exactly the guard that moved out of the `content_block_stop` match arm and into the method.

- Follow-up fix, not a regression: `StreamEnd::into_assistant_message_for_tool_use()` now returns `Some(..)` for a stream whose call was truncated mid-input, so replaying that message serializes `tool_use.input` as a JSON string, and Anthropic requires an object. The same exposure already exists for malformed non-truncated arguments.
  - Suggested change: in `into_anthropic_request_parts`, coerce a non-object `ToolCall.fn_arguments` to `{}` with a warn, so a captured truncated call cannot produce a request Anthropic rejects.

- Out of scope, unchanged: a `thinking` block cut off by `max_tokens` is still dropped, because only a block that reaches `content_block_stop` is turned into an `InterStreamThoughtBlock`.


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
