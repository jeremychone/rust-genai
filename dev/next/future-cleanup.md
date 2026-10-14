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

