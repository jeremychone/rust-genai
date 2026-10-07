//! Replay integration tests for the Bedrock Converse adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/bedrock/`, recorded against
//! gpt-5.6-terra (encrypted reasoning) and Claude Haiku 4.5 (signed thinking). Streaming cassettes
//! are raw AWS event-stream bytes (binary), as ConverseStream sends them.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

// Recorded against `global.openai.gpt-5.6-terra`, whose reasoning comes back as an encrypted
// `redactedContent` block: no reasoning text, one opaque blob per reasoning block.
const MODEL: &str = "bedrock_api::global.openai.gpt-5.6-terra";

fn reasoning_tool_request() -> ChatRequest {
	ChatRequest::new(vec![ChatMessage::user(
		"I'm planning a picnic and can't decide between Paris and Lyon, France. Think it through step by step, \
		 then check the weather (in C) for the city you'd consider first.",
	)])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": { "type": "string" },
			"country": { "type": "string" },
			"unit": { "type": "string", "enum": ["C", "F"] }
		},
		"required": ["city", "country", "unit"],
	})))
}

fn reasoning_options() -> ChatOptions {
	ChatOptions::default()
		.with_capture_content(true)
		.with_capture_tool_calls(true)
		.with_capture_reasoning_content(true)
		.with_capture_usage(true)
		.with_reasoning_effort(ReasoningEffort::Medium)
}

/// Round 1 streams a redacted reasoning block and a tool call; round 2 answers after the tool
/// result. Usage arrives in the `metadata` frame, after `messageStop`, on both rounds.
#[tokio::test]
async fn test_yakbak_bedrock_reasoning_tool_stream_round_trip() -> TestResult<()> {
	let (client, _server) = replay_client("bedrock", "reasoning_tool_stream").await?;
	let options = reasoning_options();
	let initial_request = reasoning_tool_request();
	let continuation_base = initial_request.clone();

	// -- Round 1: reasoning + tool call
	let stream_res = client.exec_chat_stream(MODEL, initial_request, Some(&options)).await?;
	let first = extract_stream_end(stream_res.stream).await?;

	assert_eq!(first.thought_signature_chunks.len(), 1);
	let blob = first.thought_signature_chunks[0].clone();
	assert!(!blob.is_empty());
	assert_eq!(
		first.stream_end.captured_thought_signatures().ok_or("captured signatures")?,
		[blob.as_str()]
	);
	// Redacted reasoning has no text.
	assert!(first.reasoning_content.is_none());

	let parts = first.stream_end.captured_content.as_ref().ok_or("captured content")?.parts();
	assert!(matches!(&parts[0], ContentPart::ThoughtSignature(sig) if *sig == blob));
	assert!(!parts.iter().any(|part| matches!(part, ContentPart::ReasoningContent(_))));
	assert!(matches!(parts.last(), Some(ContentPart::ToolCall(_))));

	let tool_calls = first.stream_end.captured_tool_calls().ok_or("tool calls")?;
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(
		tool_calls[0].thought_signatures.as_deref(),
		Some([blob.clone()].as_slice())
	);

	let usage = first.stream_end.captured_usage.as_ref().ok_or("round 1 usage")?;
	assert!(usage.prompt_tokens.unwrap_or_default() > 0);
	assert!(usage.completion_tokens.unwrap_or_default() > 0);
	assert!(matches!(
		first.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));

	// -- The replayed assistant turn carries the blob ahead of the tool call
	let tool_call = tool_calls[0].clone();
	let continuation_request = continuation_base.append_tool_use_from_stream_end(
		&first.stream_end,
		ToolResponse::from_tool_call(&tool_call, r#"{"weather": "Sunny", "temperature": "24C"}"#),
	);
	let assistant = continuation_request
		.messages
		.iter()
		.rev()
		.find(|message| message.role == ChatRole::Assistant)
		.ok_or("assistant tool-use message")?;
	assert_eq!(assistant.content.thought_signatures(), [blob.as_str()]);
	assert!(!assistant.content.contains_reasoning_content());
	assert_eq!(assistant.content.tool_calls().len(), 1);

	// -- Round 2: the answer
	let continuation_res = client.exec_chat_stream(MODEL, continuation_request, Some(&options)).await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	assert!(!continuation.content.as_deref().unwrap_or_default().is_empty());
	assert!(matches!(
		continuation.stream_end.captured_stop_reason,
		Some(StopReason::Completed(_))
	));
	let usage = continuation.stream_end.captured_usage.as_ref().ok_or("round 2 usage")?;
	assert!(usage.prompt_tokens.unwrap_or_default() > 0);
	assert!(usage.completion_tokens.unwrap_or_default() > 0);

	Ok(())
}

/// The non-streamed response keeps the redacted block as a `ThoughtSignature` part and mirrors
/// it onto the tool call, so the turn can be replayed like a streamed one.
#[tokio::test]
async fn test_yakbak_bedrock_reasoning_tool_non_stream() -> TestResult<()> {
	let (client, _server) = replay_client("bedrock", "reasoning_tool_non_stream").await?;

	let res = client
		.exec_chat(MODEL, reasoning_tool_request(), Some(&reasoning_options()))
		.await?;

	let signatures = res.content.thought_signatures();
	assert_eq!(signatures.len(), 1);
	let blob = signatures[0].to_string();
	assert!(matches!(&res.content.parts()[0], ContentPart::ThoughtSignature(_)));
	assert!(res.reasoning_content.is_none());

	let tool_calls = res.tool_calls();
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(tool_calls[0].thought_signatures.as_deref(), Some([blob].as_slice()));

	assert!(res.usage.prompt_tokens.unwrap_or_default() > 0);
	assert!(res.usage.completion_tokens.unwrap_or_default() > 0);
	assert!(matches!(res.stop_reason, Some(StopReason::ToolCall(_))));

	Ok(())
}

/// Claude Haiku with extended thinking. Round 1 streams a signed `reasoningText` block (text
/// deltas, then a signature) and a tool call; the replayed assistant turn pairs the signature with
/// its text, which the adapter sends back as `reasoningText { text, signature }`.
#[tokio::test]
async fn test_yakbak_bedrock_claude_thinking_tool_stream_round_trip() -> TestResult<()> {
	let (client, _server) = replay_client("bedrock", "claude_thinking_tool_stream").await?;
	let model = "bedrock_api::us.anthropic.claude-haiku-4-5-20251001-v1:0";
	let options = reasoning_options().with_reasoning_effort(ReasoningEffort::Low);
	let initial_request = reasoning_tool_request();
	let continuation_base = initial_request.clone();

	// -- Round 1: thinking + tool call
	let stream_res = client.exec_chat_stream(model, initial_request, Some(&options)).await?;
	let first = extract_stream_end(stream_res.stream).await?;

	let reasoning = first.reasoning_content.clone().ok_or("streamed reasoning")?;
	assert!(!reasoning.is_empty());
	assert_eq!(first.thought_signature_chunks.len(), 1);
	let signature = first.thought_signature_chunks[0].clone();

	let parts = first.stream_end.captured_content.as_ref().ok_or("captured content")?.parts();
	assert!(matches!(&parts[0], ContentPart::ThoughtSignature(sig) if *sig == signature));
	assert!(matches!(&parts[1], ContentPart::ReasoningContent(text) if *text == reasoning));
	assert!(matches!(parts.last(), Some(ContentPart::ToolCall(_))));

	let tool_calls = first.stream_end.captured_tool_calls().ok_or("tool calls")?;
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(
		tool_calls[0].thought_signatures.as_deref(),
		Some([signature.clone()].as_slice())
	);
	assert!(first.stream_end.captured_usage.is_some());
	assert!(matches!(
		first.stream_end.captured_stop_reason,
		Some(StopReason::ToolCall(_))
	));

	// -- The replayed assistant turn keeps the signature paired with its text
	let tool_call = tool_calls[0].clone();
	let continuation_request = continuation_base.append_tool_use_from_stream_end(
		&first.stream_end,
		ToolResponse::from_tool_call(&tool_call, r#"{"weather": "Sunny", "temperature": "24C"}"#),
	);
	let assistant = continuation_request
		.messages
		.iter()
		.rev()
		.find(|message| message.role == ChatRole::Assistant)
		.ok_or("assistant tool-use message")?;
	let assistant_parts = assistant.content.parts();
	assert!(matches!(&assistant_parts[0], ContentPart::ThoughtSignature(sig) if *sig == signature));
	assert!(matches!(&assistant_parts[1], ContentPart::ReasoningContent(text) if *text == reasoning));

	// -- Round 2: the answer
	let continuation_res = client.exec_chat_stream(model, continuation_request, Some(&options)).await?;
	let continuation = extract_stream_end(continuation_res.stream).await?;
	assert!(!continuation.content.as_deref().unwrap_or_default().is_empty());
	assert!(matches!(
		continuation.stream_end.captured_stop_reason,
		Some(StopReason::Completed(_))
	));
	assert!(continuation.stream_end.captured_usage.is_some());

	Ok(())
}
