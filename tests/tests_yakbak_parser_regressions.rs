//! Yakbak replay coverage for provider response parsing regressions.

mod support;

use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ContentPart, StopReason, ToolResponse};
use serde_json::json;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

// #340: retain a tool call when max_tokens ends an open Anthropic block.
#[tokio::test]
async fn anthropic_max_tokens_retains_partial_tool_call() -> TestResult<()> {
	let (client, _server) = replay_client("anthropic", "max_tokens_tool_regression").await?;
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let res = client
		.exec_chat_stream(
			"anthropic::claude-opus-5",
			ChatRequest::from_user("Write a summary"),
			Some(&options),
		)
		.await?;
	let end = extract_stream_end(res.stream).await?.stream_end;
	let calls = end.captured_tool_calls().ok_or("partial Anthropic tool call")?;
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "toolu_fixture");
	assert_eq!(calls[0].fn_arguments, json!("{\"summary\":\"The American Rev"));
	assert!(matches!(end.captured_stop_reason, Some(StopReason::MaxTokens(_))));
	Ok(())
}

// #339: retain partial Responses tool arguments at response.incomplete.
#[tokio::test]
async fn responses_incomplete_retains_partial_tool_call() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "incomplete_tool_regression").await?;
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let res = client
		.exec_chat_stream(
			"openai_resp::gpt-6-luna",
			ChatRequest::from_user("Write a summary"),
			Some(&options),
		)
		.await?;
	let end = extract_stream_end(res.stream).await?.stream_end;
	let calls = end.captured_tool_calls().ok_or("partial Responses tool call")?;
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].fn_arguments, json!("{\"summary\":\"The American Rev"));
	assert!(matches!(end.captured_stop_reason, Some(StopReason::MaxTokens(_))));
	Ok(())
}

// #332: completed arguments override a partial delta.
#[tokio::test]
async fn responses_done_arguments_override_partial_delta() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "arguments_on_done_regression").await?;
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let res = client
		.exec_chat_stream(
			"openai_resp::gpt-6-luna",
			ChatRequest::from_user("Weather"),
			Some(&options),
		)
		.await?;
	let end = extract_stream_end(res.stream).await?.stream_end;
	let calls = end.captured_tool_calls().ok_or("Responses tool call")?;
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].fn_arguments, json!({"city":"Paris"}));
	Ok(())
}

// #344: keep the nonstream reasoning summary and a single tool call.
#[tokio::test]
async fn responses_nonstream_keeps_reasoning_and_one_tool_call() -> TestResult<()> {
	let (client, _server) = replay_client("openai_resp", "nonstream_reasoning_regression").await?;
	let response = client
		.exec_chat("openai_resp::gpt-6-luna", ChatRequest::from_user("Weather"), None)
		.await?;
	let calls = response.tool_calls();
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "call_once");
	assert_eq!(calls[0].fn_arguments, json!({"city":"Paris"}));
	assert_eq!(response.reasoning_content.as_deref(), Some("Check Paris."));
	Ok(())
}

// #301: preserve OpenRouter reasoning_details in provider order.
#[tokio::test]
async fn openrouter_preserves_reasoning_details_order() -> TestResult<()> {
	let (client, _server) = replay_client("open_router", "reasoning_details_regression").await?;
	let response = client
		.exec_chat(
			"open_router::anthropic/claude-sonnet-4",
			ChatRequest::from_user("Read first"),
			None,
		)
		.await?;
	let parts = response.content.parts();
	let ContentPart::Custom(first) = &parts[0] else {
		return Err("missing first reasoning detail".into());
	};
	let ContentPart::Custom(second) = &parts[1] else {
		return Err("missing second reasoning detail".into());
	};
	assert_eq!(first.data()["signature"], "fixture-signature");
	assert_eq!(second.data()["data"], "fixture-opaque");
	assert_eq!(response.tool_calls().len(), 1);
	Ok(())
}

// #302: keep each Gemini signature next to the part it arrived on.
#[tokio::test]
async fn gemini_nonstream_keeps_signature_on_function_call() -> TestResult<()> {
	let (client, _server) = replay_client("gemini", "nonstream_signature_regression").await?;
	let response = client
		.exec_chat("gemini-3-flash", ChatRequest::from_user("Weather"), None)
		.await?;
	let calls = response.tool_calls();
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "call_gemini_fixture");
	assert_eq!(
		calls[0].thought_signatures.as_deref(),
		Some(["fixture-signature".to_string()].as_slice())
	);
	let parts = response.content.parts();
	assert!(matches!(&parts[0], ContentPart::ThoughtSignature(s) if s == "fixture-text-signature"));
	assert!(matches!(&parts[1], ContentPart::Text(s) if s == "Checking Cairo."));
	assert!(matches!(&parts[2], ContentPart::ThoughtSignature(s) if s == "fixture-signature"));
	assert!(matches!(&parts[3], ContentPart::ToolCall(_)));
	assert!(matches!(&parts[4], ContentPart::Text(s) if s == "Done."));
	Ok(())
}

// #335: the provider call ID must return on both the call and its response.
#[tokio::test]
async fn gemini_stream_keeps_provider_function_call_id() -> TestResult<()> {
	let (client, server) = replay_client("gemini", "function_call_id_regression").await?;
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let request = ChatRequest::from_user("Weather");
	let res = client
		.exec_chat_stream("gemini-3-flash", request.clone(), Some(&options))
		.await?;
	let end = extract_stream_end(res.stream).await?.stream_end;
	let calls = end.captured_tool_calls().ok_or("Gemini function call")?;
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "call_gemini_fixture");
	let call_id = calls[0].call_id.clone();
	let content = end.captured_content.ok_or("Gemini assistant content")?;
	let continuation = request
		.append_message(ChatMessage::assistant(content))
		.append_message(ToolResponse::new(call_id, "Sunny"));
	client.exec_chat("gemini-3-flash", continuation, None).await?;
	let requests = server.replay_requests().await;
	let parts = requests[1]["contents"][1]["parts"].as_array().ok_or("assistant parts")?;
	assert!(parts.iter().any(|part| part["functionCall"]["id"] == "call_gemini_fixture"));
	let parts = requests[1]["contents"][2]["parts"].as_array().ok_or("tool response parts")?;
	assert!(parts.iter().any(|part| part["functionResponse"]["id"] == "call_gemini_fixture"));
	Ok(())
}

// #328: Mistral thinking chunks remain separate from answer text.
#[tokio::test]
async fn mistral_stream_separates_thinking_and_answer() -> TestResult<()> {
	let (client, _server) = replay_client("mistral", "thinking_chunks_regression").await?;
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true);
	let res = client
		.exec_chat_stream(
			"mistral::mistral-large-4",
			ChatRequest::from_user("17 x 23?"),
			Some(&options),
		)
		.await?;
	let extract = extract_stream_end(res.stream).await?;
	assert_eq!(extract.reasoning_content.as_deref(), Some("17 x 23 = 391."));
	assert_eq!(extract.content.as_deref(), Some("391"));
	Ok(())
}
