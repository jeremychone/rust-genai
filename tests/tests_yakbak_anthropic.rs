//! Replay integration tests for the Anthropic adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/anthropic/`
//! and assert that tool call streaming events flow through correctly.

mod support;

use genai::chat::*;
use serde_json::{Value, json};
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

/// Verify that the Anthropic adapter emits incremental ToolCallChunk events
/// during streaming: one at content_block_start (name + empty args), then one
/// per content_block_delta (accumulated args as Value::String).
#[tokio::test]
async fn test_yakbak_anthropic_tool_stream() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "tool_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
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

	let stream_res = client
		.exec_chat_stream("anthropic::claude-haiku-4-5", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Verify incremental ToolCallChunk events
	let chunks = &extract.tool_call_chunks;
	assert!(
		chunks.len() >= 2,
		"Should have at least 2 tool call chunks (start + deltas), got {}",
		chunks.len()
	);

	// First chunk: tool name with empty args (from content_block_start)
	let first = &chunks[0];
	assert_eq!(first.fn_name, "get_weather", "First chunk should have tool name");
	assert_eq!(first.call_id, "toolu_01A2B3C4D5");
	assert_eq!(
		first.fn_arguments,
		Value::String(String::new()),
		"First chunk should have empty string args"
	);

	// Subsequent chunks: accumulated args as Value::String
	let last = chunks.last().unwrap();
	assert_eq!(last.fn_name, "get_weather");
	let last_args_str = last.fn_arguments.as_str().expect("Args should be Value::String");
	assert!(
		last_args_str.contains("Paris"),
		"Final accumulated args should contain 'Paris', got: {last_args_str}"
	);

	// -- Verify captured tool calls in StreamEnd (parsed JSON)
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("Should have captured tool calls")?;
	assert_eq!(tool_calls.len(), 1);

	let tc = &tool_calls[0];
	assert_eq!(tc.fn_name, "get_weather");
	assert_eq!(
		tc.fn_arguments,
		json!({"city": "Paris", "country": "France", "unit": "C"})
	);
	assert_eq!(tc.call_id, "toolu_01A2B3C4D5");

	// -- Verify usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(85));
	assert_eq!(usage.completion_tokens, Some(42));

	Ok(())
}

/// Verify that Anthropic SSE `event: ping` maps to public `ChatStreamEvent::Heartbeat`,
/// and that heartbeats do not affect content, reasoning, tools, usage, or stop reason.
#[tokio::test]
async fn test_yakbak_anthropic_ping_stream() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "ping_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Hello"),
	]);

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("anthropic::claude-haiku-4-5", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Heartbeats (cassette has two `event: ping` messages)
	assert_eq!(
		extract.heartbeat_count, 2,
		"Should yield one Heartbeat per Anthropic ping event"
	);

	// -- Content unchanged by pings
	assert_eq!(
		extract.content.as_deref(),
		Some("Hello!"),
		"Streamed content should be the text deltas only"
	);
	assert_eq!(
		extract.stream_end.captured_first_text(),
		Some("Hello!"),
		"Captured content should match streamed text"
	);

	// -- Reasoning / tools unchanged (none in this cassette)
	assert!(
		extract.reasoning_content.is_none(),
		"Should not invent reasoning content"
	);
	assert!(
		extract.stream_end.captured_reasoning_content.is_none(),
		"Should not capture reasoning content"
	);
	assert!(
		extract.tool_call_chunks.is_empty(),
		"Should not invent tool-call chunks"
	);
	assert!(
		extract.stream_end.captured_tool_calls().is_none_or(|t| t.is_empty()),
		"Should not capture tool calls"
	);

	// -- Usage and stop reason still captured
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	// message_start input_tokens (25) + message_delta output_tokens (15)
	assert_eq!(usage.prompt_tokens, Some(25));
	assert_eq!(usage.completion_tokens, Some(15));

	assert_eq!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::Completed("end_turn".to_string())),
		"Stop reason should come from message_delta, not pings"
	);

	Ok(())
}
