//! Replay integration tests for the OpenCodeGo adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/opencode_go/`
//! and verify that the adapter correctly handles both MiniMax (Anthropic protocol)
//! and OpenAI-protocol models in chat and streaming modes.

mod support;

use genai::adapter::AdapterKind;
use genai::chat::*;
use genai::ModelIden;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

/// Replay a non-streaming chat with a MiniMax model (Anthropic protocol).
///
/// Uses `opencode_go/minimax_chat` cassette recorded via the `/messages` endpoint
/// with `x-api-key` auth header. Asserts that the adapter routes to the correct
/// URL, sends the Anthropic-format request, and parses the response.
#[tokio::test]
async fn test_yakbak_opencode_go_minimax_chat() -> TestResult<()> {
	let (client, _server) = replay_client("opencode_go", "minimax_chat").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Say hello in one word"),
	]);

	let chat_res = client
		.exec_chat(
			ModelIden::new(AdapterKind::OpenCodeGo, "minimax-m2.5"),
			chat_req,
			None,
		)
		.await?;

	// Verify the response has text content
	let text = chat_res.first_text().ok_or("Should have text content")?;
	assert!(!text.is_empty(), "Response text should not be empty: {text:?}");

	Ok(())
}

/// Replay a streaming chat with a MiniMax model (Anthropic protocol).
///
/// Uses `opencode_go/minimax_stream` cassette. The Anthropic SSE stream includes
/// thinking/content blocks. Asserts that streaming events flow through correctly
/// and at least one content chunk is received.
#[tokio::test]
async fn test_yakbak_opencode_go_minimax_stream() -> TestResult<()> {
	let (client, _server) = replay_client("opencode_go", "minimax_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Say hello in one word"),
	]);

	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client
		.exec_chat_stream(
			ModelIden::new(AdapterKind::OpenCodeGo, "minimax-m2.5"),
			chat_req,
			Some(&options),
		)
		.await?;

	let extract = extract_stream_end(stream_res.stream).await?;

	// Verify content was received through the stream
	assert!(
		extract.content.is_some() && !extract.content.as_deref().unwrap_or("").is_empty(),
		"Should have received at least one content chunk"
	);

	Ok(())
}

/// Replay a non-streaming chat with an OpenAI-protocol model (GLM-5).
///
/// Uses `opencode_go/openai_chat` cassette recorded via the `/chat/completions`
/// endpoint with `Authorization: Bearer` auth header. Asserts the adapter routes to
/// the correct OpenAI-compatible URL and parses the response.
#[tokio::test]
async fn test_yakbak_opencode_go_openai_chat() -> TestResult<()> {
	let (client, _server) = replay_client("opencode_go", "openai_chat").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Say hello in one word"),
	]);

	let chat_res = client
		.exec_chat(
			ModelIden::new(AdapterKind::OpenCodeGo, "glm-5"),
			chat_req,
			None,
		)
		.await?;

	// Verify the response has text content
	let text = chat_res.first_text().ok_or("Should have text content")?;
	assert!(!text.is_empty(), "Response text should not be empty: {text:?}");

	Ok(())
}

/// Replay a streaming chat with an OpenAI-protocol model (GLM-5).
///
/// Uses `opencode_go/openai_stream` cassette. The OpenAI-compatible SSE stream
/// includes `data:` chunks with content deltas. Asserts streaming events flow
/// through correctly and at least one content chunk is received.
#[tokio::test]
async fn test_yakbak_opencode_go_openai_stream() -> TestResult<()> {
	let (client, _server) = replay_client("opencode_go", "openai_stream").await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Say hello in one word"),
	]);

	let options = ChatOptions::default().with_capture_content(true);

	let stream_res = client
		.exec_chat_stream(
			ModelIden::new(AdapterKind::OpenCodeGo, "glm-5"),
			chat_req,
			Some(&options),
		)
		.await?;

	let extract = extract_stream_end(stream_res.stream).await?;

	// Verify content was received through the stream
	assert!(
		extract.content.is_some() && !extract.content.as_deref().unwrap_or("").is_empty(),
		"Should have received at least one content chunk"
	);

	Ok(())
}
