//! Replay integration tests for the OpenAI (Chat Completions API) adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/openai/`
//! and assert that tool call streaming events flow through correctly.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

/// Regression test for the "tool_calls + finish_reason in same SSE chunk" case.
///
/// Some OpenAI-compatible providers (e.g. ZhiPu/BigModel GLM-4 series, Ollama)
/// return the full tool call and `finish_reason: "tool_calls"` in the first
/// content chunk, rather than splitting arguments across multiple delta chunks
/// like OpenAI's native Chat Completions stream.
///
/// The streamer must emit a `ToolCallChunk` event for these combined chunks
/// so downstream consumers (agent loops, stream collectors) can react to
/// tool calls in real time. Before the fix, the streamer captured the tool
/// call into `captured_data` but skipped the `InterStreamEvent::ToolCallChunk`
/// emit, so streams appeared to terminate with `NaturalEnd` and no tool calls.
#[tokio::test]
async fn test_yakbak_openai_stream_tool_call_with_finish_reason() -> TestResult<()> {
	let (client, _server) = replay_client("openai", "stream_tool_call_with_finish_reason").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the weather in Paris, France in Celsius?"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "The most likely country of this city name" },
			"unit": { "type": "string", "enum": ["C", "F"], "description": "Temperature unit" }
		},
		"required": ["city", "country", "unit"],
	})));

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream("gpt-4o-mini", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Verify that a ToolCallChunk event was emitted during streaming.
	//    This is the core regression: before the fix, the streamer captured
	//    the tool call into `captured_data` but never produced a stream event,
	//    so agent loops saw NaturalEnd with no tool calls.
	let chunks = &extract.tool_call_chunks;
	assert_eq!(
		chunks.len(),
		1,
		"Should have exactly 1 tool call chunk emitted during streaming, got {}",
		chunks.len()
	);

	let chunk = &chunks[0];
	assert_eq!(chunk.fn_name, "get_weather");
	assert_eq!(chunk.call_id, "call_yakbak_fixture_0");
	// Arguments are accumulated as Value::String during streaming.
	let chunk_args_str = chunk
		.fn_arguments
		.as_str()
		.expect("Streamed chunk args should be Value::String");
	assert!(
		chunk_args_str.contains("Paris"),
		"Streamed chunk args should contain 'Paris', got: {chunk_args_str}"
	);

	// -- Verify captured tool calls in StreamEnd (parsed JSON).
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("Should have captured tool calls")?;
	assert_eq!(tool_calls.len(), 1);

	let tc = &tool_calls[0];
	assert_eq!(tc.fn_name, "get_weather");
	assert_eq!(tc.call_id, "call_yakbak_fixture_0");
	assert_eq!(
		tc.fn_arguments,
		json!({"city": "Paris", "country": "France", "unit": "C"})
	);

	// -- Verify stop reason.
	assert_eq!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall("tool_calls".to_string()))
	);

	// -- Verify usage was captured from the second chunk (content: "" + usage).
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(163));
	assert_eq!(usage.completion_tokens, Some(11));
	assert_eq!(usage.total_tokens, Some(174));

	// -- No text content should be present (tool-only response).
	assert!(
		extract.content.is_none() || extract.content.as_deref() == Some(""),
		"Tool-only response should have no text content, got: {:?}",
		extract.content
	);

	Ok(())
}

/// Regression test for providers (e.g. Gemini's OpenAI-compatible endpoint) that omit `index` on
/// streamed tool call deltas. Two parallel calls are interleaved, with the arguments split across
/// deltas that only carry the call `id`.
#[tokio::test]
async fn test_yakbak_openai_stream_tool_calls_without_index() -> TestResult<()> {
	let (client, _server) = replay_client("openai", "stream_tool_calls_without_index").await?;

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather in Paris and the time in CET?")])
		.append_tool(Tool::new("get_weather").with_schema(json!({
			"type": "object",
			"properties": { "city": { "type": "string" }, "unit": { "type": "string" } },
		})))
		.append_tool(Tool::new("get_time").with_schema(json!({
			"type": "object",
			"properties": { "zone": { "type": "string" } },
		})));

	let options = ChatOptions::default().with_capture_tool_calls(true).with_capture_usage(true);

	let stream_res = client.exec_chat_stream("gpt-4o-mini", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- One ToolCallChunk per delta, each attributed to the call its `id` belongs to.
	let chunk_ids: Vec<&str> = extract.tool_call_chunks.iter().map(|c| c.call_id.as_str()).collect();
	assert_eq!(chunk_ids, ["call_a", "call_b", "call_a", "call_b", "call_a"]);

	// -- The captured calls keep their own parsed arguments.
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("Should have captured tool calls")?;
	assert_eq!(tool_calls.len(), 2);
	assert_eq!(tool_calls[0].call_id, "call_a");
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(tool_calls[0].fn_arguments, json!({"city": "Paris", "unit": "C"}));
	assert_eq!(tool_calls[1].call_id, "call_b");
	assert_eq!(tool_calls[1].fn_name, "get_time");
	assert_eq!(tool_calls[1].fn_arguments, json!({"zone": "CET"}));

	Ok(())
}
