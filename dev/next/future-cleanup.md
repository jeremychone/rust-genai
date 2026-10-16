# Potential Cleanup to do

> Note: Those are just notes taken while coding or reviewing/merging PRs. Each of those points needs to be reasseed. 

## 2026-10-14 - PR 332 related - openai_resp done arguments

- Extract a helper such as `tool_call_from_item(&item, args)` in `src/adapter/adapters/openai_resp/streamer.rs`. The `ToolCall` construction is currently repeated in the `OutputItemAdded`, `function_call` done, and `custom_tool_call` done paths.

- Read `item.x_get_str("type")` once into a local `item_type` in the done branch, instead of calling it several times.

- Refresh `call_id` and `name` from `output_item.done` when the entry was created from an `output_item.added` that carried empty values.

- Emit a final `ToolCallChunk` when `output_item.done` overrides the accumulated arguments, so streaming consumers see the complete arguments.

- Finalize `in_progress_tool_calls` on the `ResponseIncomplete` and EOF paths. Today `captured_tool_calls` only reads `captured_data.tool_calls`, which is populated in `ResponseCompleted`, so those accumulated calls are dropped.

- Convert `tests/openai_resp_done_arguments.rs` into a yakbak protocol-fixture cassette under `tests/data/yakbak/openai_resp/`, add a `test_yakbak_openai_resp_done_arguments_override` case in `tests/tests_yakbak_openai_resp.rs`, and delete the raw TCP test.

- In the new test, use `TestResult<()>` with `?`, and check the length before indexing `calls[0]`, or use `.first().ok_or(...)`.

- In `tests/tests_yakbak_openai_resp.rs`, replace the byte-index string slices (`&reasoning[..reasoning.len().min(60)]` and the `content` tail slice) with char-boundary-safe truncation, since the content can be multi-byte (Japanese).


## 2026-10-15 - PR 333 related - openai_resp strict test returns Result

- Context: PR 333 adds the unit test `function_tool_strict_is_sent_only_when_set` in `src/adapter/adapters/openai_resp/adapter_impl.rs`. It calls `.unwrap()` on `to_web_request_data(..)`.

- Why: the nearby tests already use the `-> Result<()>` plus `?` style, for example `test_gpt_5_6_codex_responses_endpoint_omits_prompt_cache_options`. Using it keeps the test style consistent and reports errors with context.

- Suggested change: change the test signature to return `Result<()>`, replace `.unwrap()` with `?`, and end the test with `Ok(())`.

## 2026-10-15 - PR 333 related - verify Anthropic strict doc claim

- Context: the new doc line on `Tool.strict` in `src/chat/tool/tool_base.rs` says the OpenAI Responses and Anthropic adapters leave `strict` out when it is unset.

- Why: the reviewer could not verify the Anthropic part, because the Anthropic adapter source was not in context. The claim currently comes only from the PR description.

- Suggested change: check `src/adapter/adapters/anthropic/adapter_shared.rs` (or the Anthropic tool serialization). If Anthropic does not omit an unset `strict`, correct the doc comment so it only describes the Responses behavior.

## 2026-10-15 - PR 333 related - null description on function tools

- Context: in `openai_resp` `tool_to_openai_tool`, the function tool JSON still uses `"description": description`. When the tool has no description, this serializes `null`.

- Why: this is pre-existing and was not introduced by PR 333. Some Responses-compatible backends may reject or handle a `null` description differently from an omitted field.

- Suggested change: omit `description` when it is `None`, following the same `x_insert` pattern used for `strict`. Verify first that the Responses API accepts an omitted description.

## 2026-10-15 - PR 333 related - streamer ref mut style

- Context: `src/adapter/adapters/openai_resp/streamer.rs` uses `match self.captured_data.content { Some(ref mut c) => .. }`.

- Why: the Rust10x guideline asks to avoid `ref` in patterns. This is a style cleanup only, not a panic or correctness issue.

- Suggested change: use `match self.captured_data.content.as_mut() { Some(c) => .. }` or the equivalent binding, with no behavior change. Keep the existing code otherwise untouched.

## 2026-10-15 - PR 334 related - native-tls row and comment include system-proxy

- Context: PR 334 moves reqwest's `system-proxy` into the genai feature `system-proxy`, which is in `default`. Builds with `default-features = false` lose it unless they opt in.

- Why: native-tls users are the most affected group, since the README requires `default-features = false` for them. Without a hint, corporate macOS or Windows users who rely on the OS proxy may see connection failures after upgrading.

- Suggested change:
  - In `README.md`, update the TLS table `native-tls` row to `features = ["native-tls", "system-proxy"]`.
  - In `Cargo.toml`, extend the `native-tls` comment to mention re-adding `system-proxy`.

## 2026-10-15 - PR 334 related - v0.7.0 migration guide note

- Context: PR 334 changes behavior for `default-features = false` builds. They no longer read macOS or Windows OS proxy settings unless `system-proxy` is enabled.

- Why: the `CHANGELOG.md` entry is already added under API Breaking Changes. The v0.7.0 migration guide is maintainer-owned and was not updated.

- Suggested change: add a short migration note to the v0.7.0 migration guide that shows `features = ["system-proxy"]` as the way to keep the old behavior. Keep the wording aligned with the README note.

## 2026-10-15 - PR 334 related - web_stream slice indexing

- Context: `src/webc/web_stream.rs` uses range indexing such as `raw[valid_up_to..]` and `raw[..valid_up_to]`.

- Why: the indexes come from `Utf8Error::valid_up_to()` or `raw.len()`, so they are safe in practice. Range indexing is still a panic path that a future edit could break, and this is pre-existing code.

- Suggested change: use `split_at` (with a checked boundary) or `split_off` so the split is explicit and cannot panic. Keep the behavior identical and add no comments that only restate the code.

## 2026-10-15 - PR 334 related - web_config ref style

- Context: `src/client/web_config.rs` uses `if let Some(ref ...)` in proxy handling.

- Why: the style guideline prefers `.as_ref()`. This is pre-existing and does not change behavior.

- Suggested change: replace `if let Some(ref x) = self.field` with `if let Some(x) = self.field.as_ref()` in the same places, keeping the logic unchanged.

## 2026-10-15 - PR 334 related - proxy setters overwrite each other

- Context: in `src/client/web_config.rs`, `with_proxy_url`, `with_https_proxy_url`, and `with_all_proxy_url` each overwrite the same stored proxy, so only one proxy is kept.

- Why: this is not introduced by PR 334, but it can surprise callers who set an HTTP proxy and then an HTTPS proxy and expect both to apply.

- Suggested change: decide the intended semantics first. Either document that the last setter wins, or store the proxies separately per scheme and apply them in `apply_to_builder`. Add a unit test for the chosen behavior.

## 2026-10-15 - PR 335 related - confirm Vertex live tool tests

- Context: `VertexAdapter::to_gemini_web_request_data` calls `GeminiAdapter::build_gemini_request_payload`, so PR 335 also sends `functionCall.id` and `functionResponse.id` to Vertex.

- Why: the PR author ran only the Gemini live tests. Vertex defines the same `id` fields and is expected to accept them, but this is not verified.

- Suggested change: run the live Vertex tool tests in `tests/tests_p_vertex.rs`, especially `test_tool_gemini_full_flow_ok` and `test_tool_gemini_simple_ok`, before or just after merging. If one fails with 400, apply the same name and id rule that the Gemini path uses.

## 2026-10-15 - PR 335 related - send response id only when call is known

- Context: `gemini_function_response_part` sends `functionResponse.id` whenever a name was resolved. A response built with an explicit `fn_name` (for example through `ToolResponse::from_tool_call`) still sends its id when the matching call is not in the request.

- Why: this produces a `functionResponse.id` with no matching `functionCall` in `contents`. Gemini will probably ignore it, but this has not been verified, and the case can come from trimmed history.

- Suggested change: send the response id only when `call_names` contains the response's call id. When it is present, the lookup also gives the authoritative name. This is optional hardening and does not block the merge.

## 2026-10-15 - PR 335 related - reword "as before" doc comments

- Context: the doc comments on `gemini_function_call_part`, `gemini_function_response_part`, and `gemini_call_id` in `src/adapter/adapters/gemini/adapter_impl.rs` say "as before" in several places.

- Why: "as before" describes the diff, not the code, and it ages badly once merged. Future readers will not know what "before" refers to.

- Suggested change: reword to describe the current behavior, for example "as for a call without an id". Also extend the `gemini_call_id` doc comment to say that ids from other providers or callers are sent as they are.

## 2026-10-15 - PR 335 related - embed.rs dummy serde error

- Context: `src/adapter/adapters/gemini/embed.rs`, in `to_embed_response`, uses `serde_json::from_str::<()>("").unwrap_err()` in production code to create a dummy serde error.

- Why: it cannot panic in practice, because an empty string always fails to parse. It is still an unwrap-style call in production code, and the error text does not describe the real failure.

- Suggested change: keep the real parse error from the batch attempt and return it, or construct a `custom` error with a clear message. This is pre-existing and separate from PR 335.

## 2026-10-15 - PR 335 related - unwrap_or_else for call id fallback

- Context: `body_to_gemini_chat_response` in `src/adapter/adapters/gemini/adapter_impl.rs` uses `fc.x_get("id").unwrap_or(format!(..))`.

- Why: `unwrap_or` evaluates the `format!` even when an id exists. It does not panic, but it does unnecessary work on every call.

- Suggested change: switch to `unwrap_or_else(|| format!(..))`. No behavior change.

## 2026-10-15 - PR 335 related - infallible gemini part helpers

- Context: `gemini_function_call_part` and `gemini_function_response_part` return `Result<Value>` only because `x_insert` is fallible.

- Why: the reviewer noted this is acceptable and matches the file's style. Building the object as a `serde_json::Map` would make the helpers infallible and simpler to call.

- Suggested change: optional. Build the `functionCall` and `functionResponse` objects as `serde_json::Map` values, then return `Value` directly, if this makes the callers simpler. Keep it only if it reads clearly.

## 2026-10-15 - PR 335 related - document ToolResponse fields

- Context: `docs/for-llm/api-reference-for-llm.md` lists `ToolResponse` with only `call_id` and `content`.

- Why: the type also has `fn_name`, `from_tool_call(..)`, and `with_fn_name(..)`. PR 335 makes `fn_name` important, because it now decides the Gemini `functionResponse.name`.

- Suggested change: add `fn_name`, `from_tool_call(..)`, and `with_fn_name(..)` to the `ToolResponse` section of the API reference, with a short note on why the name matters for Gemini.

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
