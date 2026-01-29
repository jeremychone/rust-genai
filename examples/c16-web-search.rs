//! Example demonstrating Anthropic's web search tool.
//!
//! This example shows how to:
//! 1. Enable web search with default settings
//! 2. Configure web search with domain filtering and usage limits
//! 3. Access citations from the response
//! 4. Track web search billing
//!
//! Requires: ANTHROPIC_API_KEY environment variable
//!
//! Run with: `cargo run --example c16-web-search`

use genai::chat::{ChatRequest, ContentPart, Tool, UserLocation, WebSearchConfig};
use genai::Client;

// Web search supported models: Claude Sonnet 4/4.5, Claude Haiku 3.5/4.5, Claude Opus 4/4.1/4.5
const MODEL: &str = "claude-sonnet-4-20250514";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();

	// -- Example 1: Simple web search
	println!("=== Example 1: Simple Web Search ===\n");

	let req = ChatRequest::from_user("What are the main new features in Rust 2024? Be brief.")
		.with_tools(vec![Tool::web_search()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	// Print the text response
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

	// -- Example 2: Configured web search with domain filtering
	println!("\n=== Example 2: Configured Web Search ===\n");

	let config = WebSearchConfig::default()
		.with_max_uses(5)
		.with_allowed_domains(vec!["rust-lang.org".into(), "docs.rs".into(), "crates.io".into()]);

	let req = ChatRequest::from_user("Find information about the async-std crate. Brief answer.")
		.with_tools(vec![Tool::web_search().with_web_search_config(config)]);

	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// Use the helper method to get all citations
	let citations = res.content.citations();
	if !citations.is_empty() {
		println!("Sources cited:");
		for citation in citations {
			println!("  - {} ({})", citation.title, citation.url);
		}
	}

	// -- Example 3: Web search with user location
	println!("\n=== Example 3: Localized Web Search ===\n");

	let location = UserLocation::default()
		.with_city("Tokyo")
		.with_country("JP") // ISO 2-letter country code
		.with_timezone("Asia/Tokyo");

	let config = WebSearchConfig::default()
		.with_max_uses(3)
		.with_user_location(location);

	let req = ChatRequest::from_user("What's the current weather like? Brief answer.")
		.with_tools(vec![Tool::web_search().with_web_search_config(config)]);

	let res = client.exec_chat(MODEL, req, None).await?;

	if let Some(text) = res.content.first_text() {
		println!("Response:\n{}\n", text);
	}

	// -- Example 4: Tracking web search usage for billing
	println!("\n=== Example 4: Usage Tracking ===\n");

	let req =
		ChatRequest::from_user("What's new in the tech world today?").with_tools(vec![Tool::web_search()]);

	let res = client.exec_chat(MODEL, req, None).await?;

	println!("Token Usage:");
	println!("  Prompt tokens: {:?}", res.usage.prompt_tokens);
	println!("  Completion tokens: {:?}", res.usage.completion_tokens);

	if let Some(details) = &res.usage.completion_tokens_details {
		if let Some(searches) = details.web_search_requests {
			println!("  Web searches performed: {}", searches);
			println!("  Estimated search cost: ${:.4}", searches as f64 * 0.01);
		}
	}

	// -- Example 5: Examining all content parts
	println!("\n=== Example 5: Content Part Analysis ===\n");

	let req =
		ChatRequest::from_user("Search for the latest Rust release notes").with_tools(vec![Tool::web_search()]);

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
			ContentPart::WebSearchToolResult(wsr) => {
				println!(
					"Part {}: WebSearchToolResult ({} results)",
					i,
					wsr.results.len()
				);
				for result in &wsr.results {
					println!("  - {}: {}", result.title, result.url);
				}
			}
			_ => {
				println!("Part {}: Other type", i);
			}
		}
	}

	Ok(())
}
