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

	let stream_res = client.exec_chat_stream("gemini-2.5-flash", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// Exact text
	assert_eq!(
		extract.content.as_deref(),
		Some(
			"The sky is blue because the Earth's atmosphere, primarily nitrogen and oxygen molecules, scatters shorter, bluer wavelengths of sunlight more efficiently than longer wavelengths, dispersing blue light across the sky."
		),
	);

	// Reasoning should be substantial (1604 chars from recorded)
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning")?;
	assert_eq!(reasoning.len(), 1604, "reasoning length should be exactly 1604 chars");
	assert!(reasoning.starts_with("**Defining Atmospheric Color**"));

	// Exact usage
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(12));
	assert_eq!(usage.completion_tokens, Some(732));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_gemini_url_context_stream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "url_context_stream").await?;

	let chat_req = ChatRequest::from_user(
		"Read https://blog.rust-lang.org/ and tell me the title of the most recent post. One sentence.",
	)
	.append_tool(Tool::new("urlContext").with_config(json!({})));

	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client.exec_chat_stream("gemini-3.7-flash", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;

	assert_eq!(usage.prompt_tokens, Some(5914), "tool-use tokens must count as input");
	assert_eq!(usage.completion_tokens, Some(209), "thoughts and output tokens");
	assert_eq!(usage.total_tokens, Some(6123));

	assert_eq!(
		usage.prompt_tokens.unwrap() + usage.completion_tokens.unwrap(),
		usage.total_tokens.unwrap(),
		"prompt + completion must reconcile with the reported total"
	);

	Ok(())
}

#[tokio::test]
async fn test_yakbak_gemini_builtin_with_functions() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "builtin_with_functions").await?;

	let chat_req = ChatRequest::from_user("Use the get_weather tool to get the current weather in Cairo, in Celsius.")
		.append_tool(Tool::new_web_search().with_config(WebSearchConfig::default()))
		.append_tool(Tool::new("urlContext").with_config(json!({})))
		.append_tool(
			Tool::new("get_weather")
				.with_description("Get the current weather for a city")
				.with_schema(json!({
					"type": "object",
					"properties": {
						"city": { "type": "string" },
						"unit": { "type": "string", "enum": ["C", "F"] }
					},
					"required": ["city", "unit"],
				})),
		);

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream("gemini-3.7-flash", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// The client-side function still gets called normally with the builtins attached.
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("Should have captured a tool call")?;
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	let args = tool_calls[0]
		.fn_arguments
		.as_object()
		.ok_or("fn_arguments should be an object")?;
	assert_eq!(args.get("city").and_then(|v| v.as_str()), Some("Cairo"));

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(90));
	assert_eq!(usage.completion_tokens, Some(98), "22 visible + 76 thoughts");
	assert_eq!(usage.total_tokens, Some(188));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_gemini_tool_stream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "tool_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a thoughtful assistant. Always reason carefully before invoking tools."),
		ChatMessage::user(
			"Of these three cities — Berlin, Cairo, Paris — exactly one is in Africa. \
			 Reason carefully about which one, then call get_weather for that city in Celsius. \
			 Walk through your reasoning explicitly.",
		),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city":    { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "The country" },
			"unit":    { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"],
	})));

	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::High)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true);

	let stream_res = client
		.exec_chat_stream("gemini-3.1-pro-preview", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- Tool call: model picked Cairo
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("Should have captured tool calls")?;
	assert_eq!(tool_calls.len(), 1, "Should be exactly one tool call");

	let first = &tool_calls[0];
	assert_eq!(first.fn_name, "get_weather");
	let args = first.fn_arguments.as_object().ok_or("fn_arguments should be an object")?;
	assert_eq!(args.get("city").and_then(|v| v.as_str()), Some("Cairo"));
	assert_eq!(args.get("unit").and_then(|v| v.as_str()), Some("C"));

	// -- Reasoning summary: pro-preview emits `thought:true` text parts
	// alongside the function call when the prompt requires real reasoning.
	let reasoning = extract.reasoning_content.as_deref().ok_or("Should have reasoning")?;
	assert!(
		reasoning.len() > 100,
		"Reasoning summary should be substantial, got {} chars",
		reasoning.len()
	);
	assert!(reasoning.contains("Cairo"), "Reasoning summary should mention Cairo");

	// -- Visible text content also streamed in this response
	let content = extract.content.as_deref().ok_or("Should have visible text content")?;
	assert!(!content.is_empty(), "Visible text content should be non-empty");

	// -- Thought signature: opaque blob attached to the first tool call for handoff
	let thought_signatures = extract
		.stream_end
		.captured_thought_signatures()
		.ok_or("Should have captured thought signatures")?;
	assert!(
		!thought_signatures.is_empty() && thought_signatures[0].len() > 100,
		"Should have a non-trivial thought signature blob"
	);
	assert!(
		first.thought_signatures.as_ref().is_some_and(|t| !t.is_empty()),
		"First tool call should carry thought_signatures for handoff"
	);

	Ok(())
}

/// Streamed multi-turn tool loop where the first tool call fails and the model retries.
/// Recorded against the live API, which accepted every turn genai rebuilt from the stream's
/// captured content (a call's signature on the call, the text's on the text, none sent bare).
#[tokio::test]
async fn test_yakbak_gemini_multi_turn_thought_signatures() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "multi_turn_thought_signatures").await?;

	let mut chat_req = ChatRequest::new(vec![
		ChatMessage::system(
			"Use the get_weather tool for weather questions. Before every tool call, tell the user in one \
			 short sentence what you are about to do. If a tool call fails, say so and retry.",
		),
		ChatMessage::user("Which of Berlin, Cairo, and Paris is in Africa? Get its current weather in Celsius."),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city":    { "type": "string", "description": "The city name" },
			"country": { "type": "string", "description": "ISO 3166-1 alpha-2 country code" },
			"unit":    { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"],
	})));
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::High)
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true);

	// -- Turns 1 and 2: a sentence, then a call. The first call's result is an error.
	let tool_results = [
		r#"{"error": "Upstream weather service timed out. Retry the same request."}"#,
		r#"{"weather": "Sunny", "temperature": "32", "unit": "C"}"#,
	];
	for (turn, result) in tool_results.into_iter().enumerate() {
		let stream_res = client
			.exec_chat_stream("gemini-3.8-flash", chat_req.clone(), Some(&options))
			.await?;
		let extract = extract_stream_end(stream_res.stream).await?;
		let content = extract.stream_end.captured_content.ok_or("each turn should capture content")?;

		assert!(
			content.first_text().is_some_and(|text| !text.is_empty()),
			"turn {turn}: sentence before the call"
		);
		let tool_calls = content.tool_calls();
		assert_eq!(tool_calls.len(), 1, "turn {turn}: one call");
		let call = tool_calls[0];
		assert_eq!(call.fn_name, "get_weather");
		assert_eq!(call.fn_arguments["city"], "Cairo");

		// The signature arrived on the functionCall part, so the call carries exactly that one,
		// and it's also among the captured signature parts.
		let signatures = content.thought_signatures();
		assert_eq!(signatures.len(), 1, "turn {turn}: one signature");
		assert_eq!(
			call.thought_signatures.as_deref(),
			Some(&[signatures[0].to_string()][..]),
			"turn {turn}: the call keeps its own signature"
		);

		let call_id = call.call_id.clone();
		chat_req = chat_req
			.append_message(ChatMessage::assistant(content))
			.append_message(ToolResponse::new(call_id, result));
	}

	// -- Turn 3: the answer. Its signature arrived on a final empty-text chunk.
	let stream_res = client.exec_chat_stream("gemini-3.8-flash", chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	let content = extract.stream_end.captured_content.ok_or("the answer should capture content")?;
	assert!(content.tool_calls().is_empty(), "no further calls");
	assert_eq!(content.thought_signatures().len(), 1, "the text signature is captured");
	let answer = content.first_text().ok_or("should answer")?;
	assert!(answer.contains("Cairo"), "should name the African city: {answer}");
	assert!(answer.contains("32"), "should use the successful tool result: {answer}");

	Ok(())
}
