//! AWS Bedrock integration tests
//!
//! These tests require the AWS Bedrock API key to be set:
//! - AWS_BEARER_TOKEN_BEDROCK (Bearer token API key)
//! - AWS_REGION (optional, defaults to us-east-1)
//!
//! LIMITATIONS with Bearer token auth:
//! - Streaming (/converse-stream) requires SigV4, not supported with Bearer token
//! - Titan models don't support system messages or tool use
//! - Anthropic/Meta models require inference profiles
//!
//! To run these tests:
//! cargo test --test tests_p_bedrock -- --nocapture

mod support;

use crate::support::TestResult;
use genai::Client;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use serial_test::serial;

// Default model for testing - using Titan Express (supports more features than Lite)
const MODEL: &str = "amazon.titan-text-express-v1";
const MODEL_NS: &str = "bedrock::amazon.titan-text-express-v1";

// Helper to check if AWS Bedrock API key is set
fn has_aws_credentials() -> bool {
	std::env::var("AWS_BEARER_TOKEN_BEDROCK").is_ok()
}

// region:    --- Basic Chat Tests

/// Test basic chat without system messages (Titan doesn't support system messages)
#[tokio::test]
#[serial(bedrock)]
async fn test_bedrock_chat_simple() -> TestResult<()> {
	if !has_aws_credentials() {
		println!("Skipping Bedrock test - AWS_BEARER_TOKEN_BEDROCK not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 2 + 2? Reply with just the number.")]);

	let result = client.exec_chat(MODEL_NS, chat_req, None).await?;
	let content = result.first_text().ok_or("Should have content")?;

	assert!(!content.is_empty(), "Content should not be empty");
	println!("Bedrock response: '{}'", content.trim());
	// Titan may return "4" or "2" (answering literally) - just check we got a number response
	let trimmed = content.trim();
	assert!(
		trimmed.contains("4") || trimmed.contains("2") || trimmed.parse::<i32>().is_ok(),
		"Response should contain a number, got: {}",
		content
	);

	Ok(())
}

/// Test chat with temperature setting
#[tokio::test]
#[serial(bedrock)]
async fn test_bedrock_chat_with_temperature() -> TestResult<()> {
	if !has_aws_credentials() {
		println!("Skipping Bedrock test - AWS_BEARER_TOKEN_BEDROCK not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Say hello")]);
	let options = ChatOptions::default().with_temperature(0.5);

	let result = client.exec_chat(MODEL_NS, chat_req, Some(&options)).await?;
	let content = result.first_text().ok_or("Should have content")?;

	assert!(!content.is_empty(), "Content should not be empty");
	println!("Bedrock response with temperature: {}", content);

	Ok(())
}

/// Test chat with max_tokens setting
#[tokio::test]
#[serial(bedrock)]
async fn test_bedrock_chat_with_max_tokens() -> TestResult<()> {
	if !has_aws_credentials() {
		println!("Skipping Bedrock test - AWS_BEARER_TOKEN_BEDROCK not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Write a very long story about a cat.")]);
	let options = ChatOptions::default().with_max_tokens(20);

	let result = client.exec_chat(MODEL_NS, chat_req, Some(&options)).await?;
	let content = result.first_text().ok_or("Should have content")?;

	assert!(!content.is_empty(), "Content should not be empty");
	// The response should be relatively short due to max_tokens limit
	println!("Bedrock response (max 20 tokens): {} chars", content.len());

	Ok(())
}

/// Test with Titan Lite model
#[tokio::test]
#[serial(bedrock)]
async fn test_bedrock_titan_lite() -> TestResult<()> {
	if !has_aws_credentials() {
		println!("Skipping Bedrock test - AWS_BEARER_TOKEN_BEDROCK not set");
		return Ok(());
	}

	let client = Client::default();
	let lite_model = "bedrock::amazon.titan-text-lite-v1";

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What is 3 + 3? Answer with just the number.")]);

	let result = client.exec_chat(lite_model, chat_req, None).await?;
	let content = result.first_text().ok_or("Should have content")?;

	assert!(!content.is_empty(), "Content should not be empty");
	println!("Titan Lite response: {}", content);

	Ok(())
}

/// Test usage statistics are returned
#[tokio::test]
#[serial(bedrock)]
async fn test_bedrock_usage_stats() -> TestResult<()> {
	if !has_aws_credentials() {
		println!("Skipping Bedrock test - AWS_BEARER_TOKEN_BEDROCK not set");
		return Ok(());
	}

	let client = Client::default();
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Hi")]);

	let result = client.exec_chat(MODEL_NS, chat_req, None).await?;

	// Check usage stats are populated
	assert!(result.usage.prompt_tokens.is_some(), "Should have prompt tokens");
	assert!(
		result.usage.completion_tokens.is_some(),
		"Should have completion tokens"
	);
	assert!(result.usage.total_tokens.is_some(), "Should have total tokens");

	println!(
		"Usage: prompt={:?}, completion={:?}, total={:?}",
		result.usage.prompt_tokens, result.usage.completion_tokens, result.usage.total_tokens
	);

	Ok(())
}

// endregion: --- Basic Chat Tests

// region:    --- Model Resolution Tests

#[tokio::test]
async fn test_bedrock_list_models() -> TestResult<()> {
	let client = Client::default();
	let models = client.all_model_names(AdapterKind::Bedrock).await?;
	assert!(!models.is_empty(), "Should have models");
	assert!(models.contains(&MODEL.to_string()), "Should contain {}", MODEL);
	println!("Bedrock models: {:?}", models);
	Ok(())
}

#[tokio::test]
async fn test_bedrock_model_resolution() -> TestResult<()> {
	// Test that Bedrock models are correctly resolved
	let models = vec![
		"anthropic.claude-3-5-sonnet-20241022-v2:0",
		"anthropic.claude-3-haiku-20240307-v1:0",
		"meta.llama3-70b-instruct-v1:0",
		"amazon.titan-text-express-v1",
		"mistral.mistral-7b-instruct-v0:2",
	];

	for model in models {
		let kind = AdapterKind::from_model(model)?;
		assert_eq!(kind, AdapterKind::Bedrock, "Model {} should resolve to Bedrock", model);
	}

	// Test explicit namespace
	let kind = AdapterKind::from_model("bedrock::some-custom-model")?;
	assert_eq!(kind, AdapterKind::Bedrock, "Namespaced model should resolve to Bedrock");

	Ok(())
}

// endregion: --- Model Resolution Tests

// region:    --- Streaming Tests (Currently not supported with Bearer token)

// NOTE: Streaming via /converse-stream endpoint requires AWS SigV4 signing
// and is NOT supported with Bearer token authentication.
// These tests are commented out until SigV4 support is added.

// #[tokio::test]
// #[serial(bedrock)]
// async fn test_bedrock_streaming() -> TestResult<()> {
//     // Streaming requires SigV4 authentication
//     Ok(())
// }

// endregion: --- Streaming Tests

// region:    --- Tool Tests (Not supported by Titan models)

// NOTE: Titan models don't support tool/function calling.
// Tool tests would need to use Anthropic models with inference profiles.

// endregion: --- Tool Tests
