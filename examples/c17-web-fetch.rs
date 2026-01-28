//! Example demonstrating Anthropic's web fetch tool.
//!
//! This example shows how to:
//! 1. Enable web fetch with default settings
//! 2. Configure web fetch with domain filtering and usage limits
//! 3. Enable citations for fetched content
//! 4. Combine web fetch with web search
//! 5. Track web fetch billing
//!
//! Requires: ANTHROPIC_API_KEY environment variable
//!
//! Run with: `cargo run --example c17-web-fetch`

use genai::chat::{ChatRequest, ContentPart, Tool, WebFetchConfig};
use genai::Client;

// Web fetch supported models: Claude Sonnet 4/4.5, Claude Haiku 3.5/4.5, Claude Opus 4/4.1/4.5
const MODEL: &str = "claude-sonnet-4-20250514";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// -- Example 1: Simple web fetch
	println!("=== Example 1: Simple Web Fetch ===\n");

	let req = ChatRequest::from_user(
		"Fetch the content from https://httpbin.org/html and summarize what you find. Be brief.",
	)
	.with_tools(vec![Tool::web_fetch()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Print the text response
	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// Check for web fetch results
	for part in res.content.parts() {
		if let ContentPart::WebFetchToolResult(wfr) = part {
			println!("Fetched URL: {}", wfr.url);
			println!("Content type: {}", wfr.content.media_type);
			println!("Content length: {} chars", wfr.content.data.len());
			if let Some(title) = &wfr.content.title {
				println!("Title: {}", title);
			}
			if let Some(retrieved_at) = &wfr.retrieved_at {
				println!("Retrieved at: {}", retrieved_at);
			}
		}
	}

	// -- Example 2: Configured web fetch with domain filtering
	println!("\n=== Example 2: Configured Web Fetch ===\n");

	let config = WebFetchConfig::default()
		.with_max_uses(3)
		.with_allowed_domains(vec!["rust-lang.org".into(), "docs.rs".into(), "crates.io".into()]);

	let req = ChatRequest::from_user(
		"Fetch the main page of docs.rs and tell me what documentation hosting features it offers. Brief answer.",
	)
	.with_tools(vec![Tool::web_fetch().with_web_fetch_config(config)]);

	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// -- Example 3: Web fetch with citations enabled
	println!("\n=== Example 3: Web Fetch with Citations ===\n");

	let config = WebFetchConfig::default()
		.with_max_uses(2)
		.with_citations_enabled();

	let req = ChatRequest::from_user(
		"Fetch https://httpbin.org/robots.txt and explain what it says. Include citations.",
	)
	.with_tools(vec![Tool::web_fetch().with_web_fetch_config(config)]);

	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// Check for text with citations
	for part in res.content.parts() {
		if let ContentPart::TextWithCitations(twc) = part {
			println!("Text with {} citations:\n{}\n", twc.citations.len(), twc.text);
			for citation in &twc.citations {
				println!("  - {} ({})", citation.title, citation.url);
			}
		}
	}

	// -- Example 4: Combined web search and web fetch
	println!("\n=== Example 4: Combined Search and Fetch ===\n");

	let req = ChatRequest::from_user(
		"Search for the official Rust documentation, then fetch the main page and summarize what topics it covers. Brief answer.",
	)
	.with_tools(vec![Tool::web_search(), Tool::web_fetch()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// -- Example 5: Tracking web fetch usage for billing
	println!("\n=== Example 5: Usage Tracking ===\n");

	let req = ChatRequest::from_user("Fetch https://httpbin.org/json and describe the JSON structure.")
		.with_tools(vec![Tool::web_fetch()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	println!("Token Usage:");
	println!("  Prompt tokens: {:?}", res.usage.prompt_tokens);
	println!("  Completion tokens: {:?}", res.usage.completion_tokens);

	if let Some(details) = &res.usage.completion_tokens_details {
		if let Some(fetches) = details.web_fetch_requests {
			println!("  Web fetches performed: {}", fetches);
		}
		if let Some(searches) = details.web_search_requests {
			println!("  Web searches performed: {}", searches);
		}
	}

	// -- Example 6: Examining all content parts
	println!("\n=== Example 6: Content Part Analysis ===\n");

	let req =
		ChatRequest::from_user("Fetch https://httpbin.org/headers and show me what headers were sent.")
			.with_tools(vec![Tool::web_fetch()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	for (i, part) in res.content.parts().iter().enumerate() {
		match part {
			ContentPart::Text(text) => {
				println!("Part {}: Text ({} chars)", i, text.len());
			}
			ContentPart::TextWithCitations(twc) => {
				println!(
					"Part {}: TextWithCitations ({} chars, {} citations)",
					i,
					twc.text.len(),
					twc.citations.len()
				);
			}
			ContentPart::ServerToolUse(stu) => {
				println!("Part {}: ServerToolUse (tool: {}, id: {})", i, stu.name, stu.id);
			}
			ContentPart::WebFetchToolResult(wfr) => {
				println!(
					"Part {}: WebFetchToolResult (url: {}, {} bytes)",
					i,
					wfr.url,
					wfr.content.data.len()
				);
			}
			ContentPart::WebSearchToolResult(wsr) => {
				println!(
					"Part {}: WebSearchToolResult ({} results)",
					i,
					wsr.results.len()
				);
			}
			_ => {
				println!("Part {}: Other type", i);
			}
		}
	}

	Ok(())
}
