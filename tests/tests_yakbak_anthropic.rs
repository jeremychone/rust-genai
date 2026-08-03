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
	assert_eq!(usage.total_tokens, Some(127));

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
	assert_eq!(usage.total_tokens, Some(40));

	assert_eq!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::Completed("end_turn".to_string())),
		"Stop reason should come from message_delta, not pings"
	);

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_usage_stream() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "usage_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Hello"),
	]);

	let options = ChatOptions::default().with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("anthropic::claude-haiku-4-5", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(19));
	assert_eq!(usage.completion_tokens, Some(34));
	assert_eq!(usage.total_tokens, Some(53));

	let details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Real Anthropic sends cache_creation even when uncached, so details should be Some")?;
	assert_eq!(details.cache_creation_tokens, Some(0));
	assert_eq!(details.cached_tokens, Some(0));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_usage_cache_stream() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "usage_cache_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Hello"),
	]);

	let options = ChatOptions::default().with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("anthropic::claude-haiku-4-5", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(
		usage.prompt_tokens,
		Some(4211),
		"prompt = input + cache_creation + cache_read"
	);
	assert_eq!(usage.completion_tokens, Some(5));
	assert_eq!(usage.total_tokens, Some(4216));

	let details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	assert_eq!(details.cache_creation_tokens, Some(0));
	assert_eq!(details.cached_tokens, Some(4202));

	let creation_details = details
		.cache_creation_details
		.as_ref()
		.ok_or("message_start breakdown should survive message_delta")?;
	assert_eq!(creation_details.ephemeral_5m_tokens, Some(0));
	assert_eq!(creation_details.ephemeral_1h_tokens, Some(0));

	Ok(())
}

fn thinking_tool_request() -> ChatRequest {
	ChatRequest::new(vec![ChatMessage::user(
		"Identify the relevant city and call the weather tool.",
	)])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": { "type": "string" },
			"country": { "type": "string" },
			"unit": { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"]
	})))
}

#[tokio::test]
async fn test_yakbak_anthropic_thinking_stream_without_capture_opt_in_captures_nothing() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_tool_stream").await?;

	// No capture opt-in: signatures still stream live, but nothing is accumulated.
	let options = ChatOptions::default();

	let stream_res = client
		.exec_chat_stream("anthropic::fixture-model", thinking_tool_request(), Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	assert!(
		!extract.thought_signature_chunks.is_empty(),
		"live signature chunks are events, not capture, so they must still be emitted"
	);
	assert!(extract.stream_end.captured_thought_signatures().is_none());
	assert!(extract.stream_end.captured_reasoning_content.is_none());
	assert!(extract.stream_end.captured_content.is_none());

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_thinking_tool_stream_round_trip_capture() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_tool_stream").await?;
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);
	let initial_request = thinking_tool_request();
	let continuation_base = initial_request.clone();

	let stream_res = client
		.exec_chat_stream("anthropic::fixture-model", initial_request, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let streamed_reasoning = extract.reasoning_content.as_deref().ok_or("streamed reasoning")?;
	assert!(!streamed_reasoning.is_empty());
	assert_eq!(
		extract.stream_end.captured_reasoning_content.as_deref(),
		Some(streamed_reasoning)
	);

	assert!(!extract.thought_signature_chunks.is_empty());
	let logical_signature = extract.thought_signature_chunks.concat();
	let captured_signatures = extract.stream_end.captured_thought_signatures().ok_or("captured signatures")?;
	assert_eq!(captured_signatures, [logical_signature.as_str()]);

	let parts = extract.stream_end.captured_content.as_ref().ok_or("captured content")?.parts();
	assert!(matches!(&parts[0], ContentPart::ThoughtSignature(_)));
	assert!(matches!(parts.last(), Some(ContentPart::ToolCall(_))));

	let tool_calls = extract.stream_end.captured_tool_calls().ok_or("tool calls")?;
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(
		tool_calls[0].thought_signatures.as_deref(),
		Some([logical_signature].as_slice())
	);

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("usage")?;
	assert!(usage.prompt_tokens.unwrap_or_default() > 0);
	assert!(usage.completion_tokens.unwrap_or_default() > 0);
	assert!(matches!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));

	let tool_call = tool_calls[0].clone();
	let continuation_request = continuation_base.append_tool_use_from_stream_end(
		&extract.stream_end,
		ToolResponse::from_tool_call(&tool_call, "25 C and clear"),
	);
	let assistant = continuation_request
		.messages
		.iter()
		.rev()
		.find(|message| message.role == ChatRole::Assistant)
		.ok_or("assistant tool-use message")?;
	assert_eq!(assistant.content.thought_signatures().len(), 1);
	assert!(assistant.content.contains_reasoning_content());
	assert_eq!(assistant.content.tool_calls().len(), 1);

	let continuation_res = client
		.exec_chat_stream("anthropic::fixture-model", continuation_request, Some(&options))
		.await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	assert!(!continuation.content.as_deref().unwrap_or_default().is_empty());
	assert!(!continuation.reasoning_content.as_deref().unwrap_or_default().is_empty());
	assert!(!continuation.thought_signature_chunks.is_empty());
	assert!(
		continuation
			.stream_end
			.captured_thought_signatures()
			.is_some_and(|signatures| !signatures.is_empty())
	);
	assert!(matches!(
		continuation.stream_end.captured_stop_reason,
		Some(StopReason::Completed(_))
	));
	assert!(continuation.stream_end.captured_usage.is_some());

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_usage_cache_creation_stream() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "usage_cache_creation_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Hello"),
	]);

	let options = ChatOptions::default().with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("anthropic::claude-haiku-4-5", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(
		usage.prompt_tokens,
		Some(4211),
		"prompt = input + cache_creation + cache_read"
	);
	assert_eq!(usage.completion_tokens, Some(5));
	assert_eq!(usage.total_tokens, Some(4216));

	let details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	assert_eq!(details.cache_creation_tokens, Some(4202));
	assert_eq!(details.cached_tokens, Some(0));

	let creation_details = details
		.cache_creation_details
		.as_ref()
		.ok_or("message_start breakdown should survive message_delta")?;
	assert_eq!(creation_details.ephemeral_5m_tokens, Some(4202));
	assert_eq!(creation_details.ephemeral_1h_tokens, Some(0));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_adjudication_two_turn_multi_tool_round_trip() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_adjudication_tool_stream").await?;
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);
	let initial_request = ChatRequest::from_user("Adjudicate the public FFmpeg H.264 source candidate.").with_tools([
		Tool::new("Read"),
		Tool::new("Grep"),
		Tool::new("Submit"),
	]);
	let continuation_base = initial_request.clone();

	let stream_res = client
		.exec_chat_stream("anthropic::fixture-model", initial_request, Some(&options))
		.await?;
	let first = extract_stream_end(stream_res.stream).await?;
	assert_eq!(first.reasoning_content.as_deref(), Some("Let me look at the source."));
	assert_eq!(first.thought_signature_chunks.len(), 1);
	assert!(!first.thought_signature_chunks[0].is_empty());
	assert_eq!(
		first.stream_end.captured_thought_signatures(),
		Some(first.thought_signature_chunks.iter().map(String::as_str).collect())
	);

	let first_content = first.stream_end.captured_content.as_ref().ok_or("captured first content")?;
	assert!(matches!(first_content.parts()[0], ContentPart::ThoughtSignature(_)));
	assert!(matches!(first_content.parts()[1], ContentPart::ReasoningContent(_)));
	assert!(matches!(first_content.parts()[2], ContentPart::ToolCall(_)));
	assert!(matches!(first_content.parts()[3], ContentPart::ToolCall(_)));

	let first_tool_calls: Vec<ToolCall> = first
		.stream_end
		.captured_tool_calls()
		.ok_or("first-turn tool calls")?
		.into_iter()
		.cloned()
		.collect();
	assert_eq!(first_tool_calls.len(), 2);
	assert!(first_tool_calls.iter().all(|call| call.fn_name == "Read"));
	assert_eq!(
		first_tool_calls[0].thought_signatures.as_deref(),
		Some(first.thought_signature_chunks.as_slice())
	);
	assert!(first_tool_calls[1].thought_signatures.is_none());
	assert!(matches!(
		first.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));
	assert!(first.stream_end.captured_usage.is_some());

	let mut continuation_request = continuation_base.append_tool_use_from_stream_end(
		&first.stream_end,
		ToolResponse::from_tool_call(&first_tool_calls[0], "public FFmpeg source excerpt 1"),
	);
	continuation_request = continuation_request.append_message(ToolResponse::from_tool_call(
		&first_tool_calls[1],
		"public FFmpeg source excerpt 2",
	));
	assert_eq!(
		continuation_request
			.messages
			.iter()
			.filter(|message| message.role == ChatRole::Assistant)
			.count(),
		1
	);
	assert_eq!(
		continuation_request
			.messages
			.iter()
			.filter(|message| message.role == ChatRole::Tool)
			.count(),
		2
	);
	let assistant = continuation_request
		.messages
		.iter()
		.find(|message| message.role == ChatRole::Assistant)
		.ok_or("assistant tool-use message")?;
	assert_eq!(assistant.content.thought_signatures().len(), 1);
	assert!(assistant.content.contains_reasoning_content());
	assert_eq!(assistant.content.tool_calls().len(), 2);

	let continuation_res = client
		.exec_chat_stream("anthropic::fixture-model", continuation_request, Some(&options))
		.await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	assert_eq!(
		continuation.reasoning_content.as_deref(),
		Some("Now let's check current_slice: is it reset between frames? Search for current_slice.")
	);
	assert_eq!(continuation.thought_signature_chunks.len(), 1);
	assert!(
		continuation
			.stream_end
			.captured_thought_signatures()
			.is_some_and(|signatures| signatures.len() == 1 && !signatures[0].is_empty())
	);
	let continuation_calls = continuation.stream_end.captured_tool_calls().ok_or("continuation tool calls")?;
	assert_eq!(continuation_calls.len(), 2);
	assert!(continuation_calls.into_iter().all(|call| call.fn_name == "Grep"));
	assert!(matches!(
		continuation.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));
	assert!(continuation.stream_end.captured_usage.is_some());

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_thinking_signature_variants_preserve_block_pairs() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_signature_variants_stream").await?;
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true);
	let initial_request = thinking_tool_request();
	let continuation_base = initial_request.clone();

	let stream_res = client
		.exec_chat_stream("anthropic::fixture-model", initial_request, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	assert_eq!(
		extract.thought_signature_chunks,
		["opaque-start-only", "opaque-prefix-", "opaque-prefix-complete"]
	);
	assert_eq!(extract.reasoning_content.as_deref(), Some("First block.Second block."));
	assert_eq!(
		extract.stream_end.captured_thought_signatures(),
		Some(vec!["opaque-start-only", "opaque-prefix-complete"])
	);

	let content = extract.stream_end.captured_content.as_ref().ok_or("captured content")?;
	let parts = content.parts();
	assert_eq!(parts.len(), 5);
	assert!(matches!(&parts[0], ContentPart::ThoughtSignature(value) if value == "opaque-start-only"));
	assert!(matches!(&parts[1], ContentPart::ReasoningContent(value) if value == "First block."));
	assert!(matches!(&parts[2], ContentPart::ThoughtSignature(value) if value == "opaque-prefix-complete"));
	assert!(matches!(&parts[3], ContentPart::ReasoningContent(value) if value == "Second block."));
	assert!(matches!(&parts[4], ContentPart::ToolCall(_)));

	let tool_call = extract.stream_end.captured_tool_calls().ok_or("tool call")?[0].clone();
	assert_eq!(
		tool_call.thought_signatures.as_deref(),
		Some(["opaque-start-only".to_string(), "opaque-prefix-complete".to_string()].as_slice())
	);
	assert_eq!(
		extract.stream_end.captured_usage.as_ref().and_then(|usage| usage.total_tokens),
		Some(20)
	);
	assert!(matches!(
		extract.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));

	let continuation_request = continuation_base.append_tool_use_from_stream_end(
		&extract.stream_end,
		ToolResponse::from_tool_call(&tool_call, "25 C and clear"),
	);
	let assistant = continuation_request
		.messages
		.iter()
		.rev()
		.find(|message| message.role == ChatRole::Assistant)
		.ok_or("assistant tool-use message")?;
	assert_eq!(
		assistant.content.thought_signatures(),
		vec!["opaque-start-only", "opaque-prefix-complete"]
	);
	assert_eq!(
		assistant.content.reasoning_contents(),
		vec!["First block.", "Second block."]
	);

	let continuation_res = client
		.exec_chat_stream("anthropic::fixture-model", continuation_request, Some(&options))
		.await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	assert_eq!(continuation.content.as_deref(), Some("Continuation accepted."));
	assert!(matches!(
		continuation.stream_end.captured_stop_reason,
		Some(StopReason::Completed(_))
	));

	Ok(())
}

#[tokio::test]
async fn test_yakbak_anthropic_thinking_tool_non_stream_capture() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "thinking_tool_non_stream").await?;
	let response = client
		.exec_chat("anthropic::fixture-model", thinking_tool_request(), None)
		.await?;

	assert!(!response.reasoning_content.as_deref().unwrap_or_default().is_empty());
	let signatures = response.content.thought_signatures();
	assert_eq!(signatures.len(), 1);
	assert!(matches!(
		response.content.parts().first(),
		Some(ContentPart::ThoughtSignature(_))
	));
	assert!(matches!(
		response.content.parts().last(),
		Some(ContentPart::ToolCall(_))
	));

	let tool_calls = response.tool_calls();
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(
		tool_calls[0]
			.thought_signatures
			.as_ref()
			.map(|values| values.iter().map(String::as_str).collect::<Vec<_>>()),
		Some(signatures)
	);
	assert!(response.usage.prompt_tokens.unwrap_or_default() > 0);
	assert!(response.usage.completion_tokens.unwrap_or_default() > 0);
	assert!(matches!(response.stop_reason, Some(StopReason::ToolCall(_))));

	let assistant = response
		.into_assistant_message_for_tool_use()
		.ok_or("assistant tool-use message")?;
	assert_eq!(assistant.content.thought_signatures().len(), 1);
	assert!(assistant.content.contains_reasoning_content());

	Ok(())
}
