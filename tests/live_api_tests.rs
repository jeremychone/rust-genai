//! Live API integration tests
//!
//! These tests run against actual Anthropic, OpenRouter, and Together.ai APIs.
//! They require valid API keys to be set in environment variables:
//! - ANTHROPIC_API_KEY for Anthropic tests
//! - OPENROUTER_API_KEY for OpenRouter tests
//! - TOGETHER_API_KEY for Together.ai tests
//!
//! To run these tests:
//! cargo test --test live_api_tests -- --ignored
//!
//! Tests will be skipped if API keys are not available.

mod support;

use genai::Client;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, Tool};
use serial_test::serial;
use support::{TestResult, extract_stream_end};

/// Helper to check if environment variable is set
fn has_env_key(key: &str) -> bool {
	std::env::var(key).is_ok_and(|v| !v.is_empty())
}

// ===== ANTHROPIC LIVE API TESTS =====

#[tokio::test]
#[serial]
#[ignore] // Ignored by default to avoid accidental API calls
async fn test_anthropic_live_basic_chat() -> TestResult<()> {
	if !has_env_key("ANTHROPIC_API_KEY") {
		println!("Skipping ANTHROPIC_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant."),
		ChatMessage::user("Say 'Hello from live test!'"),
	]);

	let result = client.exec_chat("claude-3-5-haiku-latest", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	assert!(content.contains("Hello"));
	println!("Anthropic basic chat response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_anthropic_live_tool_calling() -> TestResult<()> {
	if !has_env_key("ANTHROPIC_API_KEY") {
		println!("Skipping ANTHROPIC_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();

	let tool = Tool::new("get_weather").with_schema(serde_json::json!({
		"type": "object",
		"properties": {
			"location": {
				"type": "string",
				"description": "The city and state, e.g. San Francisco, CA"
			},
			"unit": {
				"type": "string",
				"enum": ["celsius", "fahrenheit"]
			}
		},
		"required": ["location"]
	}));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather in Paris?")]).append_tool(tool);

	let result = client.exec_chat("claude-3-5-haiku-latest", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	println!("Anthropic tool call response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_anthropic_live_streaming() -> TestResult<()> {
	if !has_env_key("ANTHROPIC_API_KEY") {
		println!("Skipping ANTHROPIC_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Count from 1 to 5 slowly")]);

	let options = ChatOptions::default().with_capture_content(true);

	let chat_res = client
		.exec_chat_stream("claude-3-5-haiku-latest", chat_req, Some(&options))
		.await?;

	let stream_extract = extract_stream_end(chat_res.stream).await?;
	let content = stream_extract.content.ok_or("Should have content")?;

	assert!(!content.is_empty());
	println!("Anthropic streaming content: {}", content);
	Ok(())
}

// ===== OPENROUTER LIVE API TESTS =====

#[tokio::test]
#[serial]
#[ignore]
async fn test_openrouter_live_basic_chat() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant."),
		ChatMessage::user("Say 'Hello from OpenRouter live test!'"),
	]);

	let result = client.exec_chat("anthropic/claude-3.5-sonnet", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	assert!(content.contains("Hello"));
	println!("OpenRouter basic chat response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_openrouter_live_tool_calling() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();

	let tool = Tool::new("get_weather").with_schema(serde_json::json!({
		"type": "object",
		"properties": {
			"location": {
				"type": "string",
				"description": "The city and state, e.g. San Francisco, CA"
			},
			"unit": {
				"type": "string",
				"enum": ["celsius", "fahrenheit"]
			}
		},
		"required": ["location"]
	}));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather in Tokyo?")]).append_tool(tool);

	let result = client.exec_chat("anthropic/claude-3.5-sonnet", chat_req, None).await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	println!("OpenRouter tool call response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_openrouter_live_streaming() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Count from 1 to 5 slowly")]);

	let options = ChatOptions::default().with_capture_content(true);

	let chat_res = client
		.exec_chat_stream("openrouter::anthropic/claude-3.5-sonnet", chat_req, Some(&options))
		.await?;

	let stream_extract = extract_stream_end(chat_res.stream).await?;
	let content = stream_extract.content.ok_or("Should have content")?;

	assert!(!content.is_empty());
	println!("OpenRouter streaming content: {}", content);
	Ok(())
}

// ===== TOGETHER.AI LIVE API TESTS =====

#[tokio::test]
#[serial]
#[ignore]
async fn test_together_live_basic_chat() -> TestResult<()> {
	if !has_env_key("TOGETHER_API_KEY") {
		println!("Skipping TOGETHER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant."),
		ChatMessage::user("Say 'Hello from Together.ai live test!'"),
	]);

	let result = client
		.exec_chat("together::meta-llama/Llama-3.2-3B-Instruct-Turbo", chat_req, None)
		.await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	assert!(content.contains("Hello"));
	println!("Together.ai basic chat response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_together_live_tool_calling() -> TestResult<()> {
	if !has_env_key("TOGETHER_API_KEY") {
		println!("Skipping TOGETHER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();

	let tool = Tool::new("get_weather").with_schema(serde_json::json!({
		"type": "object",
		"properties": {
			"location": {
				"type": "string",
				"description": "The city and state, e.g. San Francisco, CA"
			},
			"unit": {
				"type": "string",
				"enum": ["celsius", "fahrenheit"]
			}
		},
		"required": ["location"]
	}));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather in Tokyo?")]).append_tool(tool);

	let result = client
		.exec_chat("together::meta-llama/Llama-3.2-3B-Instruct-Turbo", chat_req, None)
		.await?;

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());
	println!("Together.ai tool call response: {}", content);
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_together_live_streaming() -> TestResult<()> {
	if !has_env_key("TOGETHER_API_KEY") {
		println!("Skipping TOGETHER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Count from 1 to 5 slowly")]);

	let options = ChatOptions::default().with_capture_content(true);

	let chat_res = client
		.exec_chat_stream(
			"together::meta-llama/Llama-3.2-3B-Instruct-Turbo",
			chat_req,
			Some(&options),
		)
		.await?;

	let stream_extract = extract_stream_end(chat_res.stream).await?;
	let content = stream_extract.content.ok_or("Should have content")?;

	assert!(!content.is_empty());
	println!("Together.ai streaming content: {}", content);
	Ok(())
}

// ===== CROSS-PROVIDER COMPARISON TESTS =====

#[tokio::test]
#[serial]
#[ignore]
async fn test_cross_provider_model_comparison() -> TestResult<()> {
	if !has_env_key("ANTHROPIC_API_KEY") || !has_env_key("OPENROUTER_API_KEY") || !has_env_key("TOGETHER_API_KEY") {
		println!("Skipping comparison test - missing API keys");
		return Ok(());
	}

	// Test same prompt across both providers
	let prompt = "What is 2 + 2? Answer with just the number.";

	// Anthropic
	let anthropic_client = Client::default();
	let anthropic_chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

	let anthropic_result = anthropic_client
		.exec_chat("claude-3-5-haiku-latest", anthropic_chat_req, None)
		.await?;

	// OpenRouter (using Anthropic model via OpenRouter)
	let openrouter_client = Client::default();
	let openrouter_chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

	let openrouter_result = openrouter_client
		.exec_chat("openrouter::anthropic/claude-3.5-sonnet", openrouter_chat_req, None)
		.await?;

	// Together.ai
	let together_client = Client::default();
	let together_chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);

	let together_result = together_client
		.exec_chat(
			"together::meta-llama/Llama-3.2-3B-Instruct-Turbo",
			together_chat_req,
			None,
		)
		.await?;

	// All should give similar answers
	let anthropic_content = anthropic_result.first_text().ok_or("Should have content")?;
	let openrouter_content = openrouter_result.first_text().ok_or("Should have content")?;
	let together_content = together_result.first_text().ok_or("Should have content")?;

	assert!(!anthropic_content.is_empty());
	assert!(!openrouter_content.is_empty());
	assert!(!together_content.is_empty());

	println!("Anthropic response: {}", anthropic_content);
	println!("OpenRouter response: {}", openrouter_content);
	println!("Together.ai response: {}", together_content);

	// All should contain "4" somewhere
	assert!(anthropic_content.contains("4") || anthropic_content.contains("four"));
	assert!(openrouter_content.contains("4") || openrouter_content.contains("four"));
	assert!(together_content.contains("4") || together_content.contains("four"));
	Ok(())
}

// ===== PERFORMANCE TESTS =====

#[tokio::test]
#[serial]
#[ignore]
async fn test_anthropic_live_response_time() -> TestResult<()> {
	if !has_env_key("ANTHROPIC_API_KEY") {
		println!("Skipping ANTHROPIC_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 2 + 2?")]);

	let start = std::time::Instant::now();
	let result = client.exec_chat("claude-3-5-haiku-latest", chat_req, None).await?;
	let duration = start.elapsed();

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());

	println!("Anthropic response time: {:?} for content: {}", duration, content);
	assert!(duration.as_secs() < 30, "Response should be under 30 seconds");
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_openrouter_live_response_time() -> TestResult<()> {
	if !has_env_key("OPENROUTER_API_KEY") {
		println!("Skipping OPENROUTER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 2 + 2?")]);

	let start = std::time::Instant::now();
	let result = client.exec_chat("anthropic/claude-3.5-sonnet", chat_req, None).await?;
	let duration = start.elapsed();

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());

	println!("OpenRouter response time: {:?} for content: {}", duration, content);
	assert!(duration.as_secs() < 30, "Response should be under 30 seconds");
	Ok(())
}

#[tokio::test]
#[serial]
#[ignore]
async fn test_together_live_response_time() -> TestResult<()> {
	if !has_env_key("TOGETHER_API_KEY") {
		println!("Skipping TOGETHER_API_KEY not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 2 + 2?")]);

	let start = std::time::Instant::now();
	let result = client
		.exec_chat("together::meta-llama/Llama-3.2-3B-Instruct-Turbo", chat_req, None)
		.await?;
	let duration = start.elapsed();

	let content = result.first_text().ok_or("Should have content")?;
	assert!(!content.is_empty());

	println!("Together.ai response time: {:?} for content: {}", duration, content);
	assert!(duration.as_secs() < 30, "Response should be under 30 seconds");
	Ok(())
}
