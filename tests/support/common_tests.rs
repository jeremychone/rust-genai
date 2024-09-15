use crate::get_option_value;
use crate::support::{extract_stream_end, seed_chat_req_simple, Result};
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatResponseFormat, JsonSpec};
use genai::resolver::{AuthData, AuthResolver, AuthResolverFn, IntoAuthResolverFn};
use genai::{Client, ClientConfig, ModelIden};
use serde_json::{json, Value};
use std::sync::Arc;
use value_ext::JsonValueExt;

// region:    --- Chat

pub async fn common_test_chat_simple_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	assert!(
		!get_option_value!(chat_res.content).is_empty(),
		"Content should not be empty"
	);
	let usage = chat_res.usage;
	let input_tokens = get_option_value!(usage.input_tokens);
	let output_tokens = get_option_value!(usage.output_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);

	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == input_tokens + output_tokens,
		"total_tokens should be equal to input_tokens + output_tokens"
	);

	Ok(())
}

/// Test with JSON mode enabled. This is not a structured output test.
/// - test_token: This is to avoid checking the token (due to an Ollama bug when in JSON mode, no token is returned)
pub async fn common_test_chat_json_mode_ok(model: &str, test_token: bool) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system(
			r#"Turn the user content into the most probable JSON content. 
Reply in a JSON format."#,
		),
		ChatMessage::user(
			r#"
| Model          | Maker    
| gpt-4o	       | OpenAI
| gpt-4o-mini	   | OpenAI
| llama-3.1-70B  | Meta
		"#,
		),
	]);
	let chat_options = ChatOptions::default().with_response_format(ChatResponseFormat::JsonMode);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	// Ensure tokens are still counted
	if test_token {
		// Ollama does not send back token usage when in JSON mode
		let usage = &chat_res.usage;
		let total_tokens = get_option_value!(usage.total_tokens);
		assert!(total_tokens > 0, "total_tokens should be > 0");
	}

	// Check content
	let content = chat_res.content_text_into_string().ok_or("SHOULD HAVE CONTENT")?;
	// Parse content as JSON
	let json: serde_json::Value = serde_json::from_str(&content).map_err(|err| format!("Was not valid JSON: {err}"))?;
	// Pretty print JSON
	let pretty_json = serde_json::to_string_pretty(&json).map_err(|err| format!("Was not valid JSON: {err}"))?;

	Ok(())
}

/// Test with JSON mode enabled. This is not a structured output test.
/// - test_token: This is to avoid checking the token (due to an Ollama bug when in JSON mode, no token is returned)
pub async fn common_test_chat_json_structured_ok(model: &str, test_token: bool) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system(
			r#"Turn the user content into the most probable JSON content. 
Reply in a JSON format."#,
		),
		ChatMessage::user(
			r#"
| Model          | Maker    
| gpt-4o	       | OpenAI
| gpt-4o-mini	   | OpenAI
| llama-3.1-70B  | Meta
		"#,
		),
	]);

	let json_schema = json!({
	  "type": "object",
		// "additionalProperties": false,
	  "properties": {
			"all_models": {
				"type": "array",
				"items": {
					"type": "object",
					// "additionalProperties": false,
					"properties": {
						"maker": { "type": "string" },
						"model_name": { "type": "string" }
					},
					"required": ["maker", "model_name"]
				}
			}
	  },
	  "required": ["all_models"]
	});

	let chat_options = ChatOptions::default().with_response_format(JsonSpec::new("some-schema", json_schema));

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	// Ensure tokens are still counted
	if test_token {
		// Ollama does not send back token usage when in JSON mode
		let usage = &chat_res.usage;
		let total_tokens = get_option_value!(usage.total_tokens);
		assert!(total_tokens > 0, "total_tokens should be > 0");
	}

	// Check content
	let content = chat_res.content_text_into_string().ok_or("SHOULD HAVE CONTENT")?;
	// Parse content as JSON
	let json_response: serde_json::Value =
		serde_json::from_str(&content).map_err(|err| format!("Was not valid JSON: {err}"))?;
	// Check models count
	let models: Vec<Value> = json_response.x_get("all_models")?;
	assert_eq!(3, models.len(), "Number of models");
	let first_maker: String = models.first().ok_or("No models")?.x_get("maker")?;
	assert_eq!("OpenAI", first_maker, "First maker");

	Ok(())
}

pub async fn common_test_chat_temperature_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();
	let chat_options = ChatOptions::default().with_temperature(0.);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	// -- Check
	assert!(
		!chat_res.content_text_as_str().unwrap_or("").is_empty(),
		"Content should not be empty"
	);

	Ok(())
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

pub async fn common_test_chat_stream_simple_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check no meta_usage and captured_content
	assert!(stream_end.captured_usage.is_none(), "StreamEnd should not have any meta_usage");
	assert!(
		stream_end.captured_content.is_none(),
		"StreamEnd should not have any captured_content"
	);

	Ok(())
}

pub async fn common_test_chat_stream_capture_content_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	// Should be None as not captured
	assert!(stream_end.captured_usage.is_none(), "StreamEnd should not have any meta_usage");

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	Ok(())
}

pub async fn common_test_chat_stream_capture_all_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_usage(true).with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	let meta_usage = get_option_value!(stream_end.captured_usage);

	assert!(
		get_option_value!(meta_usage.input_tokens) > 0,
		"input_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.output_tokens) > 0,
		"output_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.total_tokens) > 0,
		"total_tokens should be > 0"
	);

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	Ok(())
}

// endregion: --- Chat Stream Tests

// region:    --- With Resolvers

pub async fn common_test_resolver_auth_ok(model: &str, auth_data: AuthData) -> Result<()> {
	// -- Setup & Fixtures
	let auth_resolver = AuthResolver::from_resolver_fn(move |model_iden: ModelIden| Ok(Some(auth_data)));
	let client = Client::builder().with_auth_resolver(auth_resolver).build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	assert!(
		!get_option_value!(chat_res.content).is_empty(),
		"Content should not be empty"
	);
	let usage = chat_res.usage;
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");

	Ok(())
}

// endregion: --- With Resolvers