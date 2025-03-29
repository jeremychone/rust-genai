use crate::get_option_value;
use crate::support::data::{get_b64_duck, IMAGE_URL_JPG_DUCK};
use crate::support::{
	assert_contains, contains_checks, extract_stream_end, get_big_content, seed_chat_req_simple,
	seed_chat_req_tool_simple, validate_checks, Check, Result, StreamExtract,
};
use genai::adapter::AdapterKind;
use genai::chat::{
	CacheControl, ChatMessage, ChatOptions, ChatRequest, ChatResponseFormat, ContentPart, ImageSource, JsonSpec, Tool,
	ToolResponse,
};
use genai::resolver::{AuthData, AuthResolver, AuthResolverFn, IntoAuthResolverFn};
use genai::{Client, ClientConfig, ModelIden};
use serde_json::{json, Value};
use std::sync::Arc;
use value_ext::JsonValueExt;

// region:    --- Chat

pub async fn common_test_chat_simple_ok(model: &str, checks: Option<Check>) -> Result<()> {
	validate_checks(checks.clone(), Check::REASONING)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.content_text_as_str().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_token + comletion_token"
	);

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING) {
		let reasoning_content = chat_res
			.reasoning_content
			.as_deref()
			.ok_or("extract_stream_end SHOULD have extracted some reasoning_content")?;
		assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
		// We can assume that the reasoning content should be bigger than the content given the prompt to keep content very concise.
		assert!(
			reasoning_content.len() > content.len(),
			"Reasoning content should be > than the content"
		);
	}

	Ok(())
}

pub async fn common_test_chat_multi_system_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Be very concise"),
		ChatMessage::system("Explain with bullet points"),
		ChatMessage::user("Why is the sky blue?"),
	])
	.with_system("And end with 'Thank you'");

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	assert!(
		!get_option_value!(chat_res.content).is_empty(),
		"Content should not be empty"
	);
	let usage = chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);

	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_tokens + completion_tokens"
	);

	Ok(())
}

/// Test with JSON mode enabled. This is not a structured output test.
/// - test_token: This is to avoid checking the token (due to an Ollama bug when in JSON mode, no token is returned)
pub async fn common_test_chat_json_mode_ok(model: &str, checks: Option<Check>) -> Result<()> {
	validate_checks(checks.clone(), Check::USAGE)?;

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
	if contains_checks(checks, Check::USAGE) {
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
pub async fn common_test_chat_json_structured_ok(model: &str, checks: Option<Check>) -> Result<()> {
	validate_checks(checks.clone(), Check::USAGE)?;

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
	if contains_checks(checks, Check::USAGE) {
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

pub async fn common_test_chat_stop_sequences_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = ChatRequest::from_user("What is the capital of England?");
	let chat_options = ChatOptions::default().with_stop_sequences(vec!["London".to_string()]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, Some(&chat_options)).await?;

	let ai_content_lower = chat_res
		.content_text_as_str()
		.ok_or("Should have a AI response")?
		.to_lowercase();

	// -- Check
	assert!(
		!ai_content_lower.contains("london"),
		"Content should not contain 'London'"
	);

	Ok(())
}

pub async fn common_test_chat_reasoning_normalize_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_normalize_reasoning_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	chat_res.content_text_as_str();
	let content = chat_res.content_text_as_str().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Reasoning Content
	let reasoning_content = chat_res.reasoning_content.as_deref().ok_or("Should have reasoning_content")?;
	// Because the test is so simple, sometime ollama for example, will have `<think>\n\n</think>`
	// So we are ok if is_empty() for now
	// assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
	assert!(
		reasoning_content.len() > content.len(),
		"reasoning_content should be > than the content"
	);

	// -- Check Usage
	let usage = chat_res.usage;
	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == prompt_tokens + completion_tokens,
		"total_tokens should be equal to prompt_token + completion_tokens"
	);

	Ok(())
}

// endregion: --- Chat

// region:    --- Chat Cache

pub async fn common_test_chat_cache_simple_user_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("Give a very short summary of what each of those files are about"),
		ChatMessage::user(big_content).with_options(CacheControl::Ephemeral),
	]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.content_text_as_str().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens >= prompt_tokens + completion_tokens,
		"total_tokens should be greater to prompt_token + comletion_token because of the cached tokens"
	);

	Ok(())
}

pub async fn common_test_chat_cache_simple_system_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let big_content = get_big_content()?;
	let chat_req = ChatRequest::new(vec![
		// -- Messages (deactivate to see the differences)
		ChatMessage::system("You are a senior developer which has the following code base:"),
		ChatMessage::system(big_content).with_options(CacheControl::Ephemeral),
		ChatMessage::user("can you give a summary of each file (very concise)"),
	]);

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check Content
	let content = chat_res.content_text_as_str().ok_or("Should have content")?;
	assert!(!content.trim().is_empty(), "Content should not be empty");

	// -- Check Usage
	let usage = &chat_res.usage;

	let prompt_tokens = get_option_value!(usage.prompt_tokens);
	let completion_tokens = get_option_value!(usage.completion_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);
	let prompt_tokens_details = usage
		.prompt_tokens_details
		.as_ref()
		.ok_or("Should have prompt_tokens_details")?;
	let cache_creation_tokens = get_option_value!(prompt_tokens_details.cache_creation_tokens);
	let cached_tokens = get_option_value!(prompt_tokens_details.cached_tokens);

	assert!(
		cache_creation_tokens > 0 || cached_tokens > 0,
		"one of cache_creation_tokens or cached_tokens should be greater than 0"
	);
	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens >= prompt_tokens + completion_tokens,
		"total_tokens should be greater to prompt_token + comletion_token because of the cached tokens"
	);

	Ok(())
}

// endregion: --- Chat Cache

// region:    --- Chat Stream Tests

pub async fn common_test_chat_stream_simple_ok(model: &str, checks: Option<Check>) -> Result<()> {
	validate_checks(checks.clone(), Check::REASONING)?;

	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
	} = extract_stream_end(chat_res.stream).await?;
	let content = content.ok_or("extract_stream_end SHOULD have extracted some content")?;

	// -- Check no meta_usage and captured_content
	assert!(!content.is_empty(), "Content streamed should not be empty");
	assert!(
		stream_end.captured_usage.is_none(),
		"StreamEnd should not have any meta_usage"
	);
	assert!(
		stream_end.captured_content.is_none(),
		"StreamEnd should not have any captured_content"
	);

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING) {
		let reasoning_content =
			reasoning_content.ok_or("extract_stream_end SHOULD have extracted some reasoning_content")?;
		assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
		// We can assume that the reasoning content should be bigger than the content given the prompt to keep content very concise.
		assert!(
			reasoning_content.len() > content.len(),
			"Reasoning content should be > than the content"
		);
	}

	Ok(())
}

/// Check that the capture content flag does the capture
/// NOTE: When checking for reasoning, the captured_reasoning_content should be None in this function
pub async fn common_test_chat_stream_capture_content_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
	} = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	// Should be None as not captured
	assert!(
		stream_end.captured_usage.is_none(),
		"StreamEnd should not have any meta_usage"
	);

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	// -- Check Reasoning Content
	// Should always be none, as it was not instructed to be captured.
	assert!(
		stream_end.captured_reasoning_content.is_none(),
		"The captured_reasoning_content should be None"
	);

	Ok(())
}

pub async fn common_test_chat_stream_capture_all_ok(model: &str, checks: Option<Check>) -> Result<()> {
	validate_checks(checks.clone(), Check::REASONING)?;

	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(
			ChatOptions::default()
				.with_capture_usage(true)
				.with_capture_content(true)
				.with_capture_reasoning_content(true),
		)
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(model, chat_req.clone(), None).await?;

	// -- Extract Stream content
	let StreamExtract {
		stream_end,
		content,
		reasoning_content,
	} = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	let meta_usage = get_option_value!(stream_end.captured_usage);

	assert!(
		get_option_value!(meta_usage.prompt_tokens) > 0,
		"prompt_token should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.completion_tokens) > 0,
		"completion_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.total_tokens) > 0,
		"total_tokens should be > 0"
	);

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	let captured_content = captured_content.text_as_str().ok_or("Captured content should have a text")?;
	assert!(!captured_content.is_empty(), "captured_content.length should be > 0");

	// -- Check Reasoning Content
	if contains_checks(checks, Check::REASONING) {
		let reasoning_content = stream_end
			.captured_reasoning_content
			.ok_or("captured_reasoning_content SHOULD have extracted some reasoning_content")?;
		assert!(!reasoning_content.is_empty(), "reasoning_content should not be empty");
		// We can assume that the reasoning content should be bigger than the content given the prompt to keep content very concise.
		assert!(
			reasoning_content.len() > captured_content.len(),
			"Reasoning content should be > than the content"
		);
	}

	Ok(())
}

// endregion: --- Chat Stream Tests

// region:    --- Images

pub async fn common_test_chat_image_url_ok(model: &str) -> Result<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_image_url("image/jpeg", IMAGE_URL_JPG_DUCK),
	]));
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.content_text_as_str().ok_or("Should have text result")?;
	assert_contains(res, "duck");

	Ok(())
}

pub async fn common_test_chat_image_b64_ok(model: &str) -> Result<()> {
	// -- Setup
	let client = Client::default();

	// -- Build & Exec
	let mut chat_req = ChatRequest::default().with_system("Answer in one sentence");
	// This is similar to sending initial system chat messages (which will be cumulative with system chat messages)
	chat_req = chat_req.append_message(ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_image_base64("image/jpeg", get_b64_duck()?),
	]));
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let res = chat_res.content_text_as_str().ok_or("Should have text result")?;
	assert_contains(res, "duck");

	Ok(())
}

// endregion: --- Images

// region:    --- Tools

/// Just making the tool request, and checking the tool call response
/// `complete_check` if for LLMs that are better at giving back the unit and weather.
pub async fn common_test_tool_simple_ok(model: &str, complete_check: bool) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_tool_simple();

	// -- Exec
	let chat_res = client.exec_chat(model, chat_req, None).await?;

	// -- Check
	let mut tool_calls = chat_res.tool_calls().ok_or("Should have tool calls")?;
	let tool_call = tool_calls.pop().ok_or("Should have at least one tool call")?;
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("city")?, "Paris");
	assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("country")?, "France");
	if complete_check {
		// Note: Not all LLM will output the weather (e.g. Anthropic Haiku)
		assert_eq!(tool_call.fn_arguments.x_get_as::<&str>("unit")?, "C");
	}

	Ok(())
}

/// `complete_check` if for LLMs that are better at giving back the unit and weather.
pub async fn common_test_tool_full_flow_ok(model: &str, complete_check: bool) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let mut chat_req = seed_chat_req_tool_simple();

	// -- Exec first request to get the tool calls
	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;
	let tool_calls = chat_res.into_tool_calls().ok_or("Should have tool calls in chat_res")?;

	// -- Exec the second request
	// get the tool call id (first one)
	let first_tool_call = tool_calls.first().ok_or("Should have at least one tool call")?;
	let first_tool_call_id = &first_tool_call.call_id;
	// simulate the response
	let tool_response = ToolResponse::new(first_tool_call_id, r#"{"weather": "Sunny", "temperature": "32C"}"#);

	// Add the tool_calls, tool_response
	let chat_req = chat_req.append_message(tool_calls).append_message(tool_response);

	let chat_res = client.exec_chat(model, chat_req.clone(), None).await?;

	// -- Check
	let content = chat_res
		.content_text_as_str()
		.ok_or("Last response should be message")?
		.to_lowercase(); // lowercase because some models send "Sunny" and not "sunny"

	assert!(content.contains("paris"), "Should contain 'Paris'");
	assert!(content.contains("32"), "Should contain '32'");
	if complete_check {
		// Note: Not all LLM will output the weather (e.g. Anthropic Haiku)
		assert!(content.contains("sunny"), "Should contain 'sunny'");
	}

	Ok(())
}

// endregion: --- Tools

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

// region:    --- List

pub async fn common_test_list_models(adapter_kind: AdapterKind, contains: &str) -> Result<()> {
	let client = Client::default();

	// -- Exec
	let models = client.all_model_names(adapter_kind).await?;

	// -- Check
	assert_contains(&models, contains);

	Ok(())
}

// endregion: --- List
