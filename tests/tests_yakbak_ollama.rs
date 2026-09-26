//! Replay integration tests for the native Ollama adapter.
//!
//! The `ndjson_multi_event_stream` cassette is a regression fixture for the
//! Ollama streamer's "emit every event of an ndjson line" fix: several lines
//! carry multiple event-bearing fields (`thinking` + `content`, `content` +
//! `tool_calls`, empty `content` + multiple `tool_calls`). All of them must
//! surface — in stream order — through the full HTTP → WebStream (ndjson
//! splitting) → OllamaStreamer path, which the streamer's inline unit tests
//! bypass.
//!
//! The `cached_usage` / `cached_usage_stream` cassettes are regression fixtures
//! for the `prompt_eval_cached_count` → `Usage.prompt_tokens_details.cached_tokens`
//! normalization (https://docs.ollama.com/api/usage), covering the non-streaming
//! and streaming usage paths end-to-end. They were captured live via curl from
//! a local Ollama `llama3.2:3b` on 2026-09-25.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};
use tokio_stream::StreamExt;

// -- Expected from the cassette (tests/data/yakbak/ollama/ndjson_multi_event_stream)
const EXPECTED_REASONING: &str = "The sky is blue due to Rayleigh scattering. ";
const EXPECTED_CONTENT: &str = "Rayleigh scattering dominates the sky. Let me check the weather.";

#[tokio::test]
async fn test_yakbak_ollama_ndjson_multi_event_stream() -> TestResult<()> {
	// -- Setup & Fixtures
	let (client, _server) = replay_client("ollama", "ndjson_multi_event_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence, then check the weather."),
		ChatMessage::user("Why is the sky blue? And what is the weather in Paris?"),
	]);
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	// -- Exec
	let stream_res = client.exec_chat_stream("ollama::qwen3", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Check: every content/reasoning field must surface, including the
	//    content token that shares a line with `thinking`.
	assert_eq!(extract.reasoning_content.as_deref(), Some(EXPECTED_REASONING));
	assert_eq!(extract.content.as_deref(), Some(EXPECTED_CONTENT));

	// -- Check: all three tool calls must be emitted. Pre-fix, tool calls sharing
	//    a line with content were dropped entirely, and only the first tool call
	//    of a line was emitted.
	let tcs = &extract.tool_call_chunks;
	assert_eq!(tcs.len(), 3, "all tool calls must be emitted, got {tcs:?}");
	assert_eq!(tcs[0].call_id, "call_a");
	assert_eq!(tcs[0].fn_name, "get_weather");
	assert_eq!(tcs[0].fn_arguments, json!({"city": "Paris"}));
	assert_eq!(tcs[1].call_id, "call_b");
	assert_eq!(tcs[1].fn_name, "get_weather");
	assert_eq!(tcs[2].call_id, "call_c");
	assert_eq!(tcs[2].fn_name, "get_time");

	// -- Check: the end captures are complete as well.
	assert_eq!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::Completed("stop".to_string()))
	);
	let captured_tcs = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("tool calls should be captured")?;
	assert_eq!(captured_tcs.len(), 3);
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("usage should be captured")?;
	assert_eq!(usage.prompt_tokens, Some(21));
	assert_eq!(usage.completion_tokens, Some(40));
	assert_eq!(usage.total_tokens, Some(61));

	Ok(())
}

/// The final done line's `prompt_eval_cached_count` (tokens read from the KV
/// cache) must surface as `Usage.prompt_tokens_details.cached_tokens`, while
/// `prompt_tokens` remains the full input count (`prompt_eval_count`).
/// Ref: https://docs.ollama.com/api/usage
/// Cassette: captured live via curl from a local Ollama `llama3.2:3b` on 2026-09-25.
#[tokio::test]
async fn test_yakbak_ollama_cached_usage_stream() -> TestResult<()> {
	// -- Setup & Fixtures
	let (client, _server) = replay_client("ollama", "cached_usage_stream").await?;

	let chat_req = ChatRequest::from_user("Say hello.");
	let options = ChatOptions::default().with_capture_usage(true);

	// -- Exec
	let stream_res = client.exec_chat_stream("ollama::llama3.2:3b", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Check
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("usage should be captured")?;
	assert_eq!(
		usage.prompt_tokens,
		Some(28),
		"prompt_eval_count is the full input count"
	);
	assert_eq!(usage.completion_tokens, Some(3));
	assert_eq!(usage.total_tokens, Some(31));

	let details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("prompt_tokens_details should be captured")?;
	assert_eq!(details.cached_tokens, Some(27));

	Ok(())
}

/// Same normalization through the non-streaming path (`OllamaAdapter::into_usage`):
/// `prompt_eval_cached_count` → `prompt_tokens_details.cached_tokens`.
/// Cassette: captured live via curl from a local Ollama `llama3.2:3b` on 2026-09-25.
#[tokio::test]
async fn test_yakbak_ollama_cached_usage_non_stream() -> TestResult<()> {
	// -- Setup & Fixtures
	let (client, _server) = replay_client("ollama", "cached_usage").await?;

	let chat_req = ChatRequest::from_user("Say hello.");

	// -- Exec
	let chat_res = client.exec_chat("ollama::llama3.2:3b", chat_req, None).await?;

	// -- Check
	assert_eq!(
		chat_res.first_text(),
		Some("Hello! It's nice to meet you. Is there something I can help you with or would you like to chat?")
	);
	assert_eq!(chat_res.stop_reason, Some(StopReason::Completed("stop".to_string())));

	let usage = &chat_res.usage;
	assert_eq!(
		usage.prompt_tokens,
		Some(28),
		"prompt_eval_count is the full input count"
	);
	assert_eq!(usage.completion_tokens, Some(25));
	assert_eq!(usage.total_tokens, Some(53));

	let details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("prompt_tokens_details should be set")?;
	assert_eq!(details.cached_tokens, Some(27));
	assert_eq!(details.cache_creation_tokens, None);

	Ok(())
}

/// Asserts the full event order across lines: reasoning → content chunks →
/// tool calls → end. The buffered events of a line must be drained in stream
/// order before the next line is processed.
#[tokio::test]
async fn test_yakbak_ollama_ndjson_multi_event_order() -> TestResult<()> {
	// -- Setup & Fixtures
	let (client, _server) = replay_client("ollama", "ndjson_multi_event_stream").await?;

	let chat_req = ChatRequest::from_user("Why is the sky blue? And what is the weather in Paris?");
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	// -- Exec
	let stream_res = client.exec_chat_stream("ollama::qwen3", chat_req, Some(&options)).await?;
	let mut stream = stream_res.stream;

	let mut labels = Vec::new();
	while let Some(event) = stream.next().await {
		// Surface stream errors instead of ending the loop on them, so a mid-stream
		// failure is reported with its actual error rather than a truncated sequence.
		let event = event?;
		match event {
			ChatStreamEvent::Chunk(c) => labels.push(format!("chunk:{}", c.content)),
			ChatStreamEvent::ReasoningChunk(c) => labels.push(format!("reasoning:{}", c.content)),
			ChatStreamEvent::ToolCallChunk(tc) => {
				labels.push(format!("tool:{}:{}", tc.tool_call.call_id, tc.tool_call.fn_name))
			}
			ChatStreamEvent::End(_) => labels.push("end".to_string()),
			ChatStreamEvent::Start | ChatStreamEvent::Heartbeat | ChatStreamEvent::ThoughtSignatureChunk(_) => (),
		}
	}

	// -- Check
	assert_eq!(
		labels,
		vec![
			format!("reasoning:{EXPECTED_REASONING}"),
			"chunk:Rayleigh".to_string(),
			"chunk: scattering dominates the sky.".to_string(),
			"chunk: Let me check the weather.".to_string(),
			"tool:call_a:get_weather".to_string(),
			"tool:call_b:get_weather".to_string(),
			"tool:call_c:get_time".to_string(),
			"end".to_string(),
		]
	);

	Ok(())
}
