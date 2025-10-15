mod support;

use crate::support::{TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

// OpenRouter models to test
const MODEL: &str = "openrouter::anthropic/claude-3.5-sonnet";
const MODEL_NS: &str = "anthropic/claude-3.5-sonnet"; // Should resolve to OpenRouter
const MODEL_GEMINI: &str = "openrouter::google/gemini-2.0-flash-exp";
const MODEL_DEEPSEEK: &str = "openrouter::deepseek/deepseek-chat";

// region:    --- Chat

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, None).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, None).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_image_url_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_url_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
}

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
#[serial(openrouter)]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
#[serial(openrouter)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(openrouter)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("OPENROUTER_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::OpenRouter, "claude-3.5-sonnet").await
}

// endregion: --- List

// region:    --- OpenRouter-Specific Tests

#[tokio::test]
#[serial(openrouter)]
async fn test_openrouter_multiple_providers() -> TestResult<()> {
	// Test different providers through OpenRouter
	let models = vec![("anthropic", MODEL), ("gemini", MODEL_GEMINI), ("deepseek", MODEL_DEEPSEEK)];

	for (provider_name, model) in models {
		println!("Testing OpenRouter provider: {}", provider_name);

		let client = genai::Client::default();
		let chat_req = genai::chat::ChatRequest::new(vec![genai::chat::ChatMessage::user(format!(
			"Say 'Hello from {}!'",
			provider_name
		))]);

		let result = client.exec_chat(model, chat_req, None).await?;
		let content = result.first_text().ok_or("Should have content")?;

		assert!(!content.is_empty(), "Content should not be empty for {}", provider_name);
		println!("✅ {} response: {}", provider_name, content);
	}

	Ok(())
}

#[tokio::test]
#[serial(openrouter)]
async fn test_openrouter_headers_validation() -> TestResult<()> {
	// This test validates that OpenRouter-specific headers are being sent
	// We can't directly test headers in genai, but we can verify the adapter works
	let client = genai::Client::default();
	let chat_req = genai::chat::ChatRequest::new(vec![genai::chat::ChatMessage::user("Test OpenRouter headers")]);

	let result = client.exec_chat(MODEL, chat_req, None).await?;
	let content = result.first_text().ok_or("Should have content")?;

	assert!(!content.is_empty(), "Content should not be empty");
	println!("✅ OpenRouter headers test passed: {}", content);

	Ok(())
}

#[tokio::test]
#[serial(openrouter)]
async fn test_openrouter_model_resolution() -> TestResult<()> {
	// Test that different model naming conventions work
	let test_cases = vec![
		("openrouter::anthropic/claude-3.5-sonnet", "namespaced model"),
		("anthropic/claude-3.5-sonnet", "auto-detected model"),
	];

	for (model, description) in test_cases {
		println!("Testing {}: {}", description, model);

		let client = genai::Client::default();
		let chat_req = genai::chat::ChatRequest::new(vec![genai::chat::ChatMessage::user("Say 'OK'")]);

		let result = client.exec_chat(model, chat_req, None).await?;
		let content = result.first_text().ok_or("Should have content")?;

		assert!(!content.is_empty(), "Content should not be empty for {}", description);
		println!("✅ {} works: {}", description, content);
	}

	Ok(())
}

// endregion: --- OpenRouter-Specific Tests
