//! Replay integration tests for the AIHubMix adapter.
//!
//! These tests use pre-recorded cassettes from `tests/data/yakbak/aihubmix/`
//! and assert that content and usage flow through correctly.

mod support;

use genai::chat::*;
use support::yakbak::replay_client;
use support::{TestResult, extract_stream_end};

#[tokio::test]
async fn test_yakbak_aihubmix_chat_stream() -> TestResult<()> {
	let (client, _server) = replay_client("aihubmix", "chat_stream").await?;

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say 'hello' and nothing else.")]);
	let options = ChatOptions::default().with_capture_content(true).with_capture_usage(true);

	let stream_res = client
		.exec_chat_stream("aihubmix::gpt-4o-mini", chat_req, Some(&options))
		.await?;
	let extract = extract_stream_end(stream_res.stream).await?;

	let content = extract.content.as_deref().ok_or("Should have streamed content")?;
	assert!(
		content.to_lowercase().contains("hello"),
		"Content should contain 'hello', got: {content}"
	);

	let usage = extract.stream_end.captured_usage.as_ref().ok_or("Should have usage")?;
	assert_eq!(usage.prompt_tokens, Some(15));
	assert_eq!(usage.completion_tokens, Some(3));
	assert_eq!(usage.total_tokens, Some(18));

	Ok(())
}
