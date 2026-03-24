//! Replay integration tests for the OpenAI Responses API adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/openai_resp/`
//! and assert that content and tool calls flow through correctly.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_openai_resp_reasoning_stream() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "reasoning_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text content
	assert_eq!(
		extract.content.as_deref(),
		Some("The sky appears blue because molecules in Earth\u{2019}s atmosphere scatter shorter-wavelength sunlight, like blue and violet, more strongly than longer wavelengths, and our eyes perceive the scattered light as blue."),
		"Text should match recorded response exactly"
	);

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(21));
	assert_eq!(usage.completion_tokens, Some(51));
	assert_eq!(usage.total_tokens, Some(72));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_openai_resp_stream_tools() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "reasoning_stream_tools").await?;

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
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("openai_resp::gpt-5.4-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Reasoning summary should be captured (from response.reasoning_summary_text.delta)
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning content")?;
	assert!(reasoning.starts_with("**Getting weather for Paris**"), "reasoning should start with expected header");
	assert!(reasoning.contains("fetch the weather"), "reasoning should mention fetching weather");

	// Exactly one tool call
	let tool_calls = extract.stream_end.captured_tool_calls().ok_or("Should have tool calls")?;
	assert_eq!(tool_calls.len(), 1);

	// Exact tool call details
	let tc = &tool_calls[0];
	assert_eq!(tc.fn_name, "get_weather");
	assert_eq!(tc.fn_arguments, json!({"city": "Paris", "country": "France", "unit": "C"}));
	assert!(!tc.call_id.is_empty());

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(106));
	assert_eq!(usage.completion_tokens, Some(41));
	assert_eq!(usage.total_tokens, Some(147));

	Ok(())
}
