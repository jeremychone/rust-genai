//! Integration tests for Anthropic web search functionality.
//!
//! These tests require the ANTHROPIC_API_KEY environment variable to be set.
//! Run with: `cargo test --test tests_p_anthropic_web_search -- --ignored`

mod support;

use genai::chat::{ChatRequest, ContentPart, Tool, WebSearchConfig};
use genai::Client;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

// Web search is supported on these models
const MODEL: &str = "claude-sonnet-4-20250514";

/// Test basic web search tool serialization
#[tokio::test]
#[ignore]
async fn test_web_search_basic_ok() -> TestResult<()> {
	let client = Client::default();

	let req = ChatRequest::from_user("What is the current version of Rust? Give a brief answer.")
		.with_tools(vec![Tool::web_search()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Should have some response content
	assert!(
		res.content.first_text().is_some() || res.content.contains_text_with_citations(),
		"Expected text or text with citations in response"
	);

	Ok(())
}

/// Test web search with configuration
#[tokio::test]
#[ignore]
async fn test_web_search_with_config_ok() -> TestResult<()> {
	let client = Client::default();

	let config = WebSearchConfig::default()
		.with_max_uses(3)
		.with_allowed_domains(vec!["rust-lang.org".into(), "docs.rs".into()]);

	let req = ChatRequest::from_user("What's new in the latest Rust release? Brief answer please.")
		.with_tools(vec![Tool::web_search().with_web_search_config(config)]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Should have some response content
	assert!(!res.content.is_empty(), "Expected non-empty response");

	Ok(())
}

/// Test that web search results contain citations when used
#[tokio::test]
#[ignore]
async fn test_web_search_citations_ok() -> TestResult<()> {
	let client = Client::default();

	let req = ChatRequest::from_user("What are the main features of Rust 2024 edition? Cite your sources.")
		.with_tools(vec![Tool::web_search()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Check content parts for various web search result types
	let mut has_text_content = false;
	let mut has_server_tool_use = false;
	let mut has_web_search_result = false;
	let mut has_text_with_citations = false;

	for part in res.content.parts() {
		match part {
			ContentPart::Text(_) => has_text_content = true,
			ContentPart::ServerToolUse(stu) => {
				has_server_tool_use = true;
				assert_eq!(stu.name, "web_search", "Expected web_search tool use");
			}
			ContentPart::WebSearchToolResult(wsr) => {
				has_web_search_result = true;
				// Results should have a tool_use_id linking back to the ServerToolUse
				assert!(!wsr.tool_use_id.is_empty(), "Expected non-empty tool_use_id");
			}
			ContentPart::TextWithCitations(twc) => {
				has_text_with_citations = true;
				// Text should not be empty
				assert!(!twc.text.is_empty(), "Expected non-empty text");
				// Note: Citations may or may not be present depending on the model's response
			}
			_ => {}
		}
	}

	// At minimum we should have some form of text content
	assert!(
		has_text_content || has_text_with_citations,
		"Expected text content in response"
	);

	// Log what we found for debugging
	println!("Response analysis:");
	println!("  - has_text_content: {}", has_text_content);
	println!("  - has_server_tool_use: {}", has_server_tool_use);
	println!("  - has_web_search_result: {}", has_web_search_result);
	println!("  - has_text_with_citations: {}", has_text_with_citations);

	Ok(())
}

/// Test web search usage tracking (billing info)
#[tokio::test]
#[ignore]
async fn test_web_search_usage_tracking_ok() -> TestResult<()> {
	let client = Client::default();

	let req = ChatRequest::from_user("What is today's weather in Tokyo? Brief answer.")
		.with_tools(vec![Tool::web_search()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Check usage
	println!("Usage: {:?}", res.usage);

	// Basic usage should always be present
	assert!(res.usage.prompt_tokens.is_some(), "Expected prompt_tokens in usage");
	assert!(
		res.usage.completion_tokens.is_some(),
		"Expected completion_tokens in usage"
	);

	// Web search requests may be present in completion_tokens_details
	if let Some(details) = &res.usage.completion_tokens_details {
		if let Some(requests) = details.web_search_requests {
			println!("Web search requests: {}", requests);
			assert!(requests > 0, "Expected positive web search request count");
		}
	}

	Ok(())
}

/// Test web search with streaming
#[tokio::test]
#[ignore]
async fn test_web_search_streaming_ok() -> TestResult<()> {
	use futures::StreamExt;
	use genai::chat::{ChatOptions, ChatStreamEvent};

	let client = Client::default();

	let req = ChatRequest::from_user("What is the latest news about Rust programming language?")
		.with_tools(vec![Tool::web_search()]);

	let options = ChatOptions::default()
		.with_capture_usage(true)
		.with_capture_content(true);

	let res = client.exec_chat_stream(MODEL, req, Some(&options)).await?;
	let mut stream = res.stream;

	let mut chunks_received = 0;
	let mut stream_end: Option<genai::chat::StreamEnd> = None;

	while let Some(result) = stream.next().await {
		let event = result?;
		chunks_received += 1;

		// Process stream events
		match event {
			ChatStreamEvent::Chunk(chunk) => {
				print!("{}", chunk.content);
			}
			ChatStreamEvent::End(end) => {
				stream_end = Some(end);
			}
			_ => {}
		}
	}

	println!("\n\nTotal chunks: {}", chunks_received);
	assert!(chunks_received > 0, "Expected to receive stream chunks");

	// Check captured data
	if let Some(end) = stream_end {
		if let Some(usage) = end.captured_usage {
			println!("Captured usage: {:?}", usage);
		}
	}

	Ok(())
}
