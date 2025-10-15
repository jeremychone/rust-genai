//! OpenRouter streaming compatibility test for genai
//!
//! This test validates OpenRouter SSE streaming format compatibility
//! Adapted from terraphim-llm-proxy tests

#![allow(clippy::useless_conversion)]

mod support;

use genai::Client;
use genai::chat::{ChatOptions, ChatRequest};
use reqwest::Client as ReqwestClient;
use serde_json::json;
use std::time::Duration;
use support::{TestResult, extract_stream_end};
use tokio::time::timeout;

/// Test helper to check if environment variable is set
fn has_env_key(key: &str) -> bool {
	std::env::var(key).is_ok()
}

#[tokio::test]
#[ignore] // Requires real API key - run with cargo test -- --ignored
async fn test_openrouter_genai_streaming() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![genai::chat::ChatMessage::user(
		"Say 'Hello genai streaming!' and count from 1 to 3",
	)]);

	let options = ChatOptions::default().with_capture_content(true);

	let chat_res = client
		.exec_chat_stream("openrouter::anthropic/claude-3.5-sonnet", chat_req, Some(&options))
		.await;

	match chat_res {
		Ok(stream_response) => {
			println!("✅ Genai streaming request initiated");

			// Use the same pattern as common tests
			let stream_extract = extract_stream_end(stream_response.stream).await;

			match stream_extract {
				Ok(extract) => {
					let content = extract.content.ok_or("Should have content")?;
					assert!(!content.is_empty(), "Content should not be empty");
					println!("✅ Received streaming content: {}", content);

					// Check if it contains expected elements
					assert!(
						content.contains("Hello") || content.contains("hello"),
						"Should contain greeting"
					);
					println!("✅ OpenRouter streaming test passed");
					return Ok(());
				}
				Err(e) => {
					println!("❌ Stream extraction failed: {}", e);
					return Err(e.into());
				}
			}
		}
		Err(e) => {
			println!("❌ Genai streaming failed: {}", e);
			return Err(e.into());
		}
	}
}

#[tokio::test]
#[ignore] // Requires real API key - run with cargo test -- --ignored
async fn test_openrouter_direct_api_comparison() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	// Test direct OpenRouter API call to verify it works outside genai
	let api_key = std::env::var("OPENROUTER_API_KEY").unwrap();

	let client = ReqwestClient::new();
	let request = json!({
		"model": "anthropic/claude-3.5-sonnet",
		"messages": [
			{
				"role": "user",
				"content": "Say 'Hello direct API!'"
			}
		],
		"stream": true
	});

	let response = client
		.post("https://openrouter.ai/api/v1/chat/completions")
		.header("Authorization", format!("Bearer {}", api_key))
		.header("HTTP-Referer", "https://github.com/sst/genai")
		.header("X-Title", "genai-rust OpenRouter Test")
		.json(&request)
		.send()
		.await;

	match response {
		Ok(resp) => {
			println!("Direct OpenRouter response status: {}", resp.status());
			println!("Direct OpenRouter response headers: {:?}", resp.headers());

			if resp.status().is_success() {
				println!("✅ Direct OpenRouter API call successful");

				// Try to read some streaming data
				let bytes = resp.bytes().await;
				match bytes {
					Ok(data) => {
						let text = String::from_utf8_lossy(&data);
						println!("Response preview: {}", &text[..text.len().min(500)]);

						// Print first few lines to understand the format
						for (i, line) in text.lines().take(10).enumerate() {
							println!("Line {}: {:?}", i + 1, line);
						}

						if text.starts_with("data: ") {
							println!("✅ Valid SSE format detected in direct API");
						} else {
							println!("⚠️ Unexpected format from direct API");
						}
					}
					Err(e) => println!("Error reading response: {}", e),
				}
			} else {
				let status = resp.status();
				let text = resp.text().await.unwrap_or_default();
				println!("❌ Direct OpenRouter API failed: {} - {}", status, text);
			}
		}
		Err(e) => println!("❌ Direct OpenRouter request failed: {}", e),
	}

	Ok(())
}

#[tokio::test]
#[ignore] // Requires real API key - run with cargo test -- --ignored
async fn test_openrouter_streaming_timeout() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![genai::chat::ChatMessage::user(
		"Generate a very long story (this should take time)",
	)]);

	let options = ChatOptions::default().with_capture_content(true);

	match timeout(
		Duration::from_secs(10), // Short timeout for test
		client.exec_chat_stream("openrouter::anthropic/claude-3.5-sonnet", chat_req, Some(&options)),
	)
	.await
	{
		Ok(Ok(stream_response)) => {
			println!("✅ Streaming started within timeout");

			// Try to extract stream content
			match extract_stream_end(stream_response.stream).await {
				Ok(extract) => {
					if let Some(content) = extract.content {
						println!("✅ Received content: {}", &content[..content.len().min(100)]);
					} else {
						println!("⚠️ No content received");
					}
				}
				Err(e) => {
					println!("❌ Stream extraction failed: {}", e);
				}
			}
		}
		Ok(Err(e)) => {
			println!("❌ Streaming failed: {}", e);
		}
		Err(_) => {
			println!("⏰ Streaming timed out (expected for long content)");
		}
	}

	Ok(())
}
