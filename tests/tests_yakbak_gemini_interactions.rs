//! Replay integration tests for the Gemini Interactions adapter.
//!
//! These use pre-recorded cassettes from `tests/data/yakbak/gemini_interactions/` and lock in the
//! parts of the protocol that are bespoke to this adapter — and, in two cases, that the API
//! reference documents incorrectly (both were found only by calling the live API):
//!
//! - A streamed tool call has to be assembled across three event types; `interaction.completed`
//!   carries no steps to recover it from.
//! - A replayed `function_call` must be preceded by its `thought` step, so the signature has to
//!   survive on the `ToolCall` itself.
//! - `function_result.name` is required despite being marked optional.
//! - An unstored interaction reports "no id" two different ways: an absent field on the
//!   non-streaming response, and an empty string on the `interaction.completed` SSE frame.

mod support;

use genai::chat::*;
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

const MODEL: &str = "gemini-3.5-flash";

fn tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a thoughtful assistant. Always reason carefully before invoking tools."),
		ChatMessage::user(
			"Of these three cities — Berlin, Cairo, Paris — exactly one is in Africa. \
			 Reason carefully about which one, then call get_weather for that city in Celsius.",
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
	})))
}

/// The streaming tool path, which has the most bespoke logic in the adapter: `step.start` carries
/// the function name and id, `arguments_delta` streams the arguments as a partial JSON *string*,
/// and `step.stop` is the only place the call can be assembled — `interaction.completed` carries
/// no `steps` at all.
#[tokio::test]
async fn test_yakbak_gemini_ix_tool_stream() -> TestResult<()> {
	let (client, _server) = replay_client("gemini_interactions", "tool_stream").await?;

	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true)
		.with_capture_usage(true);

	let stream_res = client.exec_chat_stream(MODEL, tool_request(), Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	// -- The tool call, reassembled from step.start + arguments_delta + step.stop
	let tool_calls = extract
		.stream_end
		.captured_tool_calls()
		.ok_or("should have captured a tool call")?;
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].call_id, "call_63237");
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	// Accumulated from the `arguments_delta` string deltas, then parsed back into an object.
	assert_eq!(tool_calls[0].fn_arguments["city"], "Cairo");
	assert_eq!(tool_calls[0].fn_arguments["country"], "Egypt");
	assert_eq!(tool_calls[0].fn_arguments["unit"], "C");

	// -- The thought signature must ride the tool call, or the next turn is a 400
	let signatures = tool_calls[0]
		.thought_signatures
		.as_deref()
		.ok_or("the tool call should carry the turn's thought signature")?;
	assert_eq!(signatures.len(), 1);
	assert!(!signatures[0].is_empty());
	assert_eq!(extract.thought_signature_chunks.len(), 1, "one thought_signature delta");

	// -- `requires_action` means the model is waiting on a client-side function_result
	assert!(
		matches!(extract.stream_end.captured_stop_reason, Some(StopReason::ToolCall(_))),
		"stop_reason: {:?}",
		extract.stream_end.captured_stop_reason
	);

	// -- Unstored: the SSE frame carries `"id": ""`, which must never surface as a response id
	assert_eq!(
		extract.stream_end.captured_response_id, None,
		"an empty id must not escape as a continuable response_id"
	);

	// -- Usage: thought tokens fold into completion_tokens, broken out in the details
	let usage = extract.stream_end.captured_usage.as_ref().ok_or("should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(136));
	assert_eq!(usage.completion_tokens, Some(28 + 220));
	assert_eq!(
		usage.completion_tokens_details.as_ref().and_then(|d| d.reasoning_tokens),
		Some(220)
	);
	assert_eq!(usage.total_tokens, Some(384));

	Ok(())
}

/// Two-turn tool round trip. Turn 2 is the request shape that returned a bare 400 until the
/// thought step and the `function_result.name` were both worked out.
#[tokio::test]
async fn test_yakbak_gemini_ix_tool_full_flow() -> TestResult<()> {
	let (client, _server) = replay_client("gemini_interactions", "tool_full_flow").await?;

	let chat_req = tool_request();

	// -- Turn 1
	let chat_res = client.exec_chat(MODEL, chat_req.clone(), None).await?;
	assert!(
		matches!(chat_res.stop_reason, Some(StopReason::ToolCall(_))),
		"stop_reason: {:?}",
		chat_res.stop_reason
	);
	// This cassette was recorded when `store` defaulted to false, so the response carries no id —
	// here the field is absent entirely, unlike the streaming frame's empty string.
	assert_eq!(chat_res.response_id, None);

	let tool_calls = chat_res.into_tool_calls();
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(tool_calls[0].fn_arguments["city"], "Cairo");
	// `into_tool_calls()` drops the standalone ThoughtSignature part, so this is the only copy
	// that survives into the next turn.
	assert_eq!(
		tool_calls[0].thought_signatures.as_ref().map(Vec::len),
		Some(1),
		"the signature must survive into_tool_calls()"
	);

	// -- Turn 2: `ToolResponse::new` carries no fn_name, so the adapter recovers it from the call.
	let tool_response = ToolResponse::new(&tool_calls[0].call_id, r#"{"weather": "Sunny", "temperature": "32C"}"#);
	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	let chat_res = client.exec_chat(MODEL, chat_req, None).await?;

	// -- Check
	let content = chat_res.first_text().ok_or("turn 2 should answer")?.to_lowercase();
	assert!(content.contains("cairo"), "should name the African city");
	assert!(content.contains("sunny"), "should use the tool result");
	assert!(content.contains("32"), "should use the tool result");

	Ok(())
}

/// Server-side history — the reason this adapter exists. Turn 2 sends only the new message plus
/// the previous interaction id; the transcript never leaves the server.
#[tokio::test]
async fn test_yakbak_gemini_ix_stateful_session() -> TestResult<()> {
	let (client, _server) = replay_client("gemini_interactions", "stateful_session").await?;

	// -- Turn 1: `store` defaults to true; set explicitly so the test states its own requirement.
	let chat_req = ChatRequest::from_user("My favorite language is Rust. Reply with just 'noted'.").with_store(true);
	let res_1 = client.exec_chat(MODEL, chat_req, None).await?;

	let interaction_id = res_1.response_id.clone().ok_or("a stored interaction carries an id")?;
	assert_eq!(
		interaction_id,
		"v1_ChdjYkdWYXFyR0dxdW5qckVQaUthLTRRURIXY2JHVmFxckdHcXVuanJFUGlLYS00UVE"
	);
	let turn_1_prompt_tokens = res_1.usage.prompt_tokens.ok_or("should have usage")?;
	assert_eq!(turn_1_prompt_tokens, 14);

	// -- Turn 2: only the new message travels.
	let chat_req = ChatRequest::from_user("What is my favorite language?")
		.with_previous_response_id(&interaction_id)
		.with_store(true);
	let res_2 = client.exec_chat(MODEL, chat_req, None).await?;

	// -- Check
	assert_eq!(res_2.first_text(), Some("Your favorite language is Rust."));
	// Each turn is its own interaction resource, so the id differs from turn 1's.
	assert_ne!(res_2.response_id.as_deref(), Some(interaction_id.as_str()));

	// The proof that the server supplied the history: turn 2's prompt is an order of magnitude
	// larger than what the client actually sent.
	let turn_2_prompt_tokens = res_2.usage.prompt_tokens.ok_or("should have usage")?;
	assert!(
		turn_2_prompt_tokens > turn_1_prompt_tokens * 5,
		"turn 2 billed {turn_2_prompt_tokens} prompt tokens vs turn 1's {turn_1_prompt_tokens} — \
		 the server replayed a history the client never resent"
	);

	Ok(())
}
