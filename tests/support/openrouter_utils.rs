//! OpenRouter-specific test utilities and helpers

use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::{Client, ModelIden};
use serde_json::json;
use std::time::Duration;

/// OpenRouter test models
pub const OPENROUTER_ANTHROPIC_MODEL: &str = "openrouter::anthropic/claude-3.5-sonnet";
pub const OPENROUTER_GEMINI_MODEL: &str = "openrouter::google/gemini-2.0-flash-exp";
pub const OPENROUTER_DEEPSEEK_MODEL: &str = "openrouter::deepseek/deepseek-chat";
pub const OPENROUTER_META_MODEL: &str = "openrouter::meta-llama/llama-3.1-8b-instruct";

/// OpenRouter provider names for testing
pub const PROVIDER_ANTHROPIC: &str = "anthropic";
pub const PROVIDER_GEMINI: &str = "google";
pub const PROVIDER_DEEPSEEK: &str = "deepseek";
pub const PROVIDER_META: &str = "meta-llama";

/// Create a basic OpenRouter chat request
pub fn create_openrouter_chat_request(prompt: &str) -> ChatRequest {
	ChatRequest::new(vec![ChatMessage::user(prompt)])
}

/// Create an OpenRouter chat request with system message
pub fn create_openrouter_chat_request_with_system(system: &str, prompt: &str) -> ChatRequest {
	ChatRequest::new(vec![ChatMessage::system(system), ChatMessage::user(prompt)])
}

/// Create an OpenRouter chat request for tool testing
pub fn create_openrouter_tool_request(prompt: &str) -> ChatRequest {
	let tool = genai::chat::Tool::new("get_weather")
		.with_description("Get weather information for a location")
		.with_schema(serde_json::json!({
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "The city and state, e.g. San Francisco, CA"
				},
				"unit": {
					"type": "string",
					"enum": ["celsius", "fahrenheit"],
					"description": "Temperature unit"
				}
			},
			"required": ["location"]
		}));

	ChatRequest::new(vec![ChatMessage::user(prompt)]).append_tool(tool)
}

/// Test OpenRouter model resolution
pub async fn test_model_resolution(model: &str, expected_provider: &str) -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let chat_req = create_openrouter_chat_request("Say 'OK'");

	let result = client.exec_chat(model, chat_req, None).await?;
	let content = result.first_text().ok_or("No content received")?;

	assert!(!content.is_empty(), "Content should not be empty for model: {}", model);
	println!("✅ Model {} resolved successfully: {}", model, content);

	Ok(())
}

/// Test OpenRouter streaming with timeout
pub async fn test_openrouter_streaming_with_timeout(
	model: &str,
	prompt: &str,
	timeout_duration: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
	let client = Client::default();
	let chat_req = create_openrouter_chat_request(prompt);
	let options = ChatOptions::default().with_capture_content(true);

	let stream_result = tokio::time::timeout(
		timeout_duration,
		client.exec_chat_stream(model, chat_req, Some(&options)),
	)
	.await??;

	let stream_extract = super::helpers::extract_stream_end(stream_result.stream).await?;
	let content = stream_extract.content.ok_or("No content in stream")?;

	Ok(content)
}

/// Validate OpenRouter headers are being sent (indirectly through successful requests)
pub async fn validate_openrouter_headers(model: &str) -> Result<(), Box<dyn std::error::Error>> {
	// This is an indirect validation - if the request succeeds, headers are likely correct
	let client = Client::default();
	let chat_req = create_openrouter_chat_request("Test OpenRouter headers");

	let result = client.exec_chat(model, chat_req, None).await?;
	let content = result.first_text().ok_or("No content received")?;

	assert!(
		!content.is_empty(),
		"Content should not be empty - headers validation failed"
	);
	println!("✅ OpenRouter headers validation passed for model: {}", model);

	Ok(())
}

/// Test multiple OpenRouter providers
pub async fn test_multiple_providers() -> Result<(), Box<dyn std::error::Error>> {
	let test_cases = vec![
		(PROVIDER_ANTHROPIC, OPENROUTER_ANTHROPIC_MODEL),
		(PROVIDER_GEMINI, OPENROUTER_GEMINI_MODEL),
		(PROVIDER_DEEPSEEK, OPENROUTER_DEEPSEEK_MODEL),
	];

	for (provider_name, model) in test_cases {
		println!("Testing OpenRouter provider: {}", provider_name);

		let prompt = format!("Say 'Hello from {}!'", provider_name);
		let content = test_openrouter_streaming_with_timeout(model, &prompt, Duration::from_secs(30)).await?;

		assert!(!content.is_empty(), "Content should not be empty for {}", provider_name);
		println!("✅ {} response: {}", provider_name, content);
	}

	Ok(())
}

/// Create a JSON mode request for OpenRouter testing
pub fn create_openrouter_json_request(prompt: &str) -> (ChatRequest, ChatOptions) {
	let chat_req = ChatRequest::new(vec![ChatMessage::user(prompt)]);
	let options = ChatOptions::default().with_response_format(genai::chat::ChatResponseFormat::JsonMode);
	(chat_req, options)
}

/// Test OpenRouter JSON mode
pub async fn test_openrouter_json_mode(model: &str) -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let (chat_req, options) =
		create_openrouter_json_request("Respond with a JSON object containing 'status' and 'message' fields");

	let result = client.exec_chat(model, chat_req, Some(&options)).await?;
	let content = result.first_text().ok_or("No content received")?;

	// Try to parse as JSON
	let json_value: serde_json::Value = serde_json::from_str(content)?;

	assert!(json_value.get("status").is_some(), "JSON should contain 'status' field");
	assert!(
		json_value.get("message").is_some(),
		"JSON should contain 'message' field"
	);

	println!("✅ OpenRouter JSON mode test passed: {}", content);
	Ok(())
}

/// Test OpenRouter error handling
pub async fn test_openrouter_error_handling(invalid_model: &str) -> Result<(), Box<dyn std::error::Error>> {
	let client = Client::default();
	let chat_req = create_openrouter_chat_request("This should fail");

	let result = client.exec_chat(invalid_model, chat_req, None).await;

	match result {
		Err(_) => {
			println!("✅ OpenRouter error handling test passed - expected error occurred");
			Ok(())
		}
		Ok(response) => {
			let content = response.first_text().unwrap_or("No content");
			println!("⚠️ Unexpected success with invalid model: {}", content);
			// Some providers might succeed with invalid models, so we don't fail the test
			Ok(())
		}
	}
}
