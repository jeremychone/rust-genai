//! Replay integration tests for the Gemini adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/gemini/`
//! and assert that thinking content, tool calls, and usage flow through correctly.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_gemini_thinking_nostream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "thinking_nostream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Low);

	let chat_res = client
		.exec_chat("gemini-2.5-flash", chat_req, Some(&options))
		.await?;

	// Exact text
	assert_eq!(
		chat_res.first_text().as_deref(),
		Some("The sky is blue because sunlight scatters off molecules in Earth's atmosphere, with blue light (which has shorter wavelengths) scattering more efficiently in all directions than other colors."),
	);

	// Exact reasoning content
	assert_eq!(
		chat_res.reasoning_content.as_deref(),
		Some("**My Concise Explanation of the Sky's Color**\n\nAlright, a single sentence, as requested. The user, being the expert they are, just needs the core concept. Let's see... the question is why the sky is blue. My thought process here is to hit the key points directly: Sunlight, the atmosphere, scattering, and the crucial element of wavelength. The first draft: \"The sky is blue because sunlight is scattered by molecules in Earth's atmosphere, and blue light (which has shorter wavelengths) is scattered more efficiently than other colors.\" Yup, that works. It's all there: sunlight interacting with the atmosphere and the wavelength-dependent nature of Rayleigh scattering, which explains why blue is the dominant color we see. Length is fine; clarity is good. Delivered.\n"),
	);

	// Exact usage
	assert_eq!(chat_res.usage.prompt_tokens, Some(12));
	assert_eq!(chat_res.usage.completion_tokens, Some(169));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_gemini_thinking_stream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "thinking_stream").await?;

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
		.exec_chat_stream("gemini-2.5-flash", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text
	assert_eq!(
		extract.content.as_deref(),
		Some("The sky is blue because molecules in Earth's atmosphere scatter shorter-wavelength blue light from the sun more efficiently than longer-wavelength red light."),
	);

	// Reasoning should be substantial (862 chars from recorded)
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning")?;
	assert_eq!(reasoning.len(), 862, "reasoning length should be exactly 862 chars");
	assert!(reasoning.starts_with("**Defining Sky Color**"));

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(12));
	assert_eq!(usage.completion_tokens, Some(230));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_gemini_stream_tools() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "thinking_stream_tools").await?;

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
		.exec_chat_stream("gemini-2.5-flash", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exactly one tool call
	let tool_calls = extract.stream_end.captured_tool_calls().ok_or("Should have tool calls")?;
	assert_eq!(tool_calls.len(), 1);

	// Exact tool call
	let tc = &tool_calls[0];
	assert_eq!(tc.fn_name, "get_weather");
	assert_eq!(tc.fn_arguments, json!({"country": "France", "unit": "C", "city": "Paris"}));

	// Exact reasoning content
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning")?;
	assert!(reasoning.starts_with("**Gathering Weather Information**"));
	assert!(reasoning.contains("Refining Data Retrieval"));
	assert!(reasoning.ends_with("\n\n\n"));

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(107));
	assert_eq!(usage.completion_tokens, Some(135));

	Ok(())
}
