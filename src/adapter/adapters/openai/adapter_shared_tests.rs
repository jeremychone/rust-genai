use super::{OpenAIAdapter, ToWebRequestDataOptions};
use crate::adapter::AdapterKind;
use crate::chat::{
	Binary, CacheControl, ChatMessage, ChatOptions, ChatOptionsSet, ChatRequest, ContentPart, MessageContent, Tool,
	ToolCall, ToolChoice, ToolResponse,
};
use crate::resolver::{AuthData, Endpoint};
use crate::{ModelIden, ServiceTarget};
use serde_json::{Value, json};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_cache_control_without_eligible_content_does_not_fail_chat_completion() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
		call_id: "call_1".to_string(),
		fn_name: "get_weather".to_string(),
		fn_arguments: json!({}),
		thought_signatures: None,
	})]))
	.with_options(CacheControl::Ephemeral);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("hello"), assistant_msg]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");

	Ok(())
}

#[test]
fn test_extra_body_merged_into_chat_completion_payload() -> Result<()> {
	// -- Setup & Fixtures
	let chat_options = ChatOptions::default()
		.with_temperature(0.2)
		.with_extra_body(json!({"temperature": 0.7, "enable_thinking": false}));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		model: test_model(),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
		None,
	)?;

	// -- Check
	assert_eq!(web_req.payload["enable_thinking"], false);
	assert_eq!(web_req.payload["temperature"], 0.7);

	Ok(())
}

#[test]
fn test_tool_choice_specific_tool_serialized_on_chat_completion_payload() -> Result<()> {
	// -- Setup & Fixtures
	let chat_options = ChatOptions::default().with_tool_choice(ToolChoice::tool("get_weather"));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		model: test_model(),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let chat_req = ChatRequest::from_user("weather").with_tools(vec![Tool::new("get_weather")]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		chat_req,
		options_set,
		None,
	)?;

	// -- Check
	assert_eq!(
		web_req.payload["tool_choice"],
		json!({
			"type": "function",
			"function": { "name": "get_weather" }
		})
	);

	Ok(())
}

#[test]
fn test_null_usage_is_treated_as_absent_usage() -> Result<()> {
	// -- Setup & Fixtures
	let usage = OpenAIAdapter::into_usage(AdapterKind::OpenAI, Value::Null);

	// -- Exec & Check
	assert!(usage.prompt_tokens.is_none());
	assert!(usage.completion_tokens.is_none());
	assert!(usage.total_tokens.is_none());

	Ok(())
}

/// When an assistant message carries reasoning_content, it must appear
/// in the serialized JSON so providers that require it (Kimi, DeepSeek)
/// don't reject the request.
#[test]
fn test_reasoning_content_serialized_on_assistant_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_call = ToolCall {
		call_id: "call_1".to_string(),
		fn_name: "get_weather".to_string(),
		fn_arguments: serde_json::json!({"city": "Paris"}),
		thought_signatures: None,
	};

	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![
		ContentPart::Text("Let me check.".to_string()),
		ContentPart::ToolCall(tool_call),
	]))
	.with_reasoning_content(Some("I should look up the weather.".to_string()));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("What's the weather in Paris?"), assistant_msg]);

	// -- Exec
	let parts = OpenAIAdapter::into_openai_request_parts(&test_model(), chat_req, None)?;

	// -- Check
	// The assistant message is the second message (after user)
	let assistant_json = parts
		.messages
		.get(1)
		.ok_or_else(|| std::io::Error::other("assistant message should be present"))?;
	assert_eq!(assistant_json["role"], "assistant");
	assert_eq!(
		assistant_json["reasoning_content"], "I should look up the weather.",
		"reasoning_content should be present in serialized assistant message"
	);

	Ok(())
}

/// When reasoning_content is None, the field should not appear in the JSON.
#[test]
fn test_no_reasoning_content_when_absent() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello"), ChatMessage::assistant("Hi there!")]);

	// -- Exec
	let parts = OpenAIAdapter::into_openai_request_parts(&test_model(), chat_req, None)?;

	// -- Check
	let assistant_json = parts
		.messages
		.get(1)
		.ok_or_else(|| std::io::Error::other("assistant message should be present"))?;
	assert_eq!(assistant_json["role"], "assistant");
	assert!(
		assistant_json.get("reasoning_content").is_none(),
		"reasoning_content should be absent when not set"
	);

	Ok(())
}

#[test]
fn test_gpt_5_6_chat_completion_defaults_to_explicit_cache_mode() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		ChatRequest::from_user("hello"),
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");
	assert!(web_req.payload["prompt_cache_options"].get("ttl").is_none());
	assert!(web_req.payload["messages"][0]["content"]["prompt_cache_breakpoint"].is_null());

	Ok(())
}

#[test]
fn test_gpt_5_6_chat_completion_cache_key_uses_api_default_mode() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.6-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let options = ChatOptions::default().with_prompt_cache_key("stable-key");
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
		None,
	)?;

	// -- Check
	assert!(web_req.payload.get("prompt_cache_options").is_none());
	assert!(web_req.payload["messages"][0]["content"]["prompt_cache_breakpoint"].is_null());

	Ok(())
}

#[test]
fn test_gpt_5_6_chat_completion_places_breakpoint_on_last_eligible_block() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let chat_req = ChatRequest::new(vec![
		ChatMessage::user(vec![
			ContentPart::from_text("stable text"),
			ContentPart::from_binary_url("image/png", "https://example.com/image.png", None),
			ContentPart::from_text("last text"),
		])
		.with_options(CacheControl::Ephemeral),
	]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let blocks = web_req.payload["messages"][0]["content"]
		.as_array()
		.ok_or_else(|| std::io::Error::other("message content should be an array"))?;
	let first = blocks.first().ok_or_else(|| std::io::Error::other("missing first block"))?;
	assert!(first["prompt_cache_breakpoint"].is_null());

	let image = blocks.get(1).ok_or_else(|| std::io::Error::other("missing image block"))?;
	assert!(image["prompt_cache_breakpoint"].is_null());

	let last = blocks.get(2).ok_or_else(|| std::io::Error::other("missing last block"))?;
	assert_eq!(last["prompt_cache_breakpoint"]["mode"], "explicit");

	Ok(())
}

#[test]
fn test_gpt_5_5_chat_completion_keeps_legacy_cache_retention() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.5"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral24h);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
		None,
	)?;

	// -- Check
	assert_eq!(web_req.payload["prompt_cache_retention"], "24h");
	assert!(web_req.payload.get("prompt_cache_options").is_none());

	Ok(())
}

#[test]
fn test_gpt_5_6_chat_completion_ignores_tool_cache_control() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	};
	let chat_req = ChatRequest::from_user("hello")
		.append_tool(Tool::new("get_weather").with_cache_control(CacheControl::Ephemeral));

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target,
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");
	assert!(web_req.payload["tools"][0].get("prompt_cache_breakpoint").is_none());

	Ok(())
}

// region:    --- Managed Thinking

#[test]
fn test_managed_body_thinking_disables_zero_effort() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default().with_reasoning_effort(crate::chat::ReasoningEffort::Zero);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let payload = payload("test-model", options_set, Some(managed_options()))?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "disabled");
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

#[test]
fn test_managed_body_thinking_enables_max_effort() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default().with_reasoning_effort(crate::chat::ReasoningEffort::Max);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let payload = payload("test-model", options_set, Some(managed_options()))?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert_eq!(payload["reasoning_effort"], "max");

	Ok(())
}

#[test]
fn test_managed_body_thinking_enables_keyword_effort() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default().with_reasoning_effort(crate::chat::ReasoningEffort::Low);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let payload = payload("test-model", options_set, Some(managed_options()))?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert_eq!(payload["reasoning_effort"], "low");

	Ok(())
}

#[test]
fn test_managed_body_thinking_preserves_budget_behavior() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default().with_reasoning_effort(crate::chat::ReasoningEffort::Budget(1024));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let payload = payload("test-model", options_set, Some(managed_options()))?;

	// -- Check
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

#[test]
fn test_managed_body_thinking_omits_fields_without_effort() -> Result<()> {
	// -- Setup & Fixtures
	let options_set = ChatOptionsSet::default();

	// -- Exec
	let payload = payload("test-model", options_set, Some(managed_options()))?;

	// -- Check
	assert!(payload.get("thinking").is_none());
	assert!(payload.get("reasoning_effort").is_none());

	Ok(())
}

#[test]
fn test_managed_body_disabled_thinking_preserves_reasoning_effort_payload() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default().with_reasoning_effort(crate::chat::ReasoningEffort::Max);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));

	// -- Exec
	let payload = payload("test-model", options_set, None)?;

	// -- Check
	assert!(payload.get("thinking").is_none());
	assert_eq!(payload["reasoning_effort"], "max");

	Ok(())
}

#[test]
fn test_managed_body_thinking_uses_model_name_derived_effort() -> Result<()> {
	// -- Setup & Fixtures
	let candidates = ["test-model-high", "test-model:high", "test-model@high"];

	// -- Exec
	let (model_name, derived_effort) = candidates
		.into_iter()
		.find_map(|model_name| {
			let (effort, _) = crate::chat::ReasoningEffort::from_model_name(model_name);
			effort.map(|effort| (model_name, effort))
		})
		.ok_or_else(|| std::io::Error::other("a supported model-name reasoning suffix should be available"))?;
	let payload = payload(model_name, ChatOptionsSet::default(), Some(managed_options()))?;

	// -- Check
	assert!(matches!(derived_effort, crate::chat::ReasoningEffort::High));
	assert_eq!(payload["thinking"]["type"], "enabled");
	assert_eq!(payload["reasoning_effort"], "high");

	Ok(())
}

// endregion: --- Managed Thinking

/// Tool-result images cannot ride inside a Chat Completions `tool` message: the tool
/// message keeps its text, and the images from a run of consecutive tool messages are
/// batched into ONE follow-up `user` message, emitted before the next non-tool message.
#[test]
fn test_tool_response_image_parts_batched_into_followup_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response_1 = ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64(
		"image/png",
		"BASE64PNG",
		None,
	)]);
	let tool_response_2 =
		ToolResponse::new("call_2", "chart built").with_parts([Binary::from_base64("image/jpeg", "BASE64JPEG", None)]);
	let chat_req = ChatRequest::new(vec![
		ChatMessage::from(tool_response_1),
		ChatMessage::from(tool_response_2),
		ChatMessage::user("continue"),
	]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 4, "2 tool + 1 batched image user + 1 user");
	assert_eq!(
		messages[0],
		json!({"role": "tool", "content": "screenshot taken", "tool_call_id": "call_1"})
	);
	assert_eq!(
		messages[1],
		json!({"role": "tool", "content": "chart built", "tool_call_id": "call_2"})
	);
	assert_eq!(
		messages[2],
		json!({
			"role": "user",
			"content": [
				{"type": "text", "text": "Attached image(s) from tool result:"},
				{"type": "image_url", "image_url": {"url": "data:image/png;base64,BASE64PNG"}},
				{"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,BASE64JPEG"}},
			]
		}),
		"images from the run of tool messages must batch into one follow-up user message"
	);
	assert_eq!(messages[3], json!({"role": "user", "content": "continue"}));

	Ok(())
}

/// An image-only tool response (no text) gets the "(see attached image)" placeholder
/// as the tool message content.
#[test]
fn test_tool_response_image_only_uses_placeholder_text() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	assert_eq!(
		web_req.payload["messages"][0],
		json!({"role": "tool", "content": "(see attached image)", "tool_call_id": "call_1"})
	);
	assert_eq!(
		web_req.payload["messages"][1]["content"][1]["image_url"]["url"],
		json!("data:image/png;base64,PNG64")
	);

	Ok(())
}

/// Regression guard: a text-only `ToolResponse` must serialize exactly as before,
/// with no follow-up user message.
#[test]
fn test_tool_response_text_only_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 1, "no follow-up user message for text-only");
	assert_eq!(
		messages[0],
		json!({"role": "tool", "content": "42", "tool_call_id": "call_1"})
	);

	Ok(())
}

// region:    --- Embedded Tool Responses

/// A `ToolResponse` embedded in a User-role message (Anthropic-style user-carried
/// tool result) must be extracted as a proper `role:"tool"` message placed BEFORE
/// the user message carrying the remaining content, adjacent to the assistant
/// `tool_calls` message that conventionally precedes it.
#[test]
fn test_user_embedded_tool_response_extracted_before_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
		call_id: "call_1".to_string(),
		fn_name: "get_weather".to_string(),
		fn_arguments: json!({"city": "Paris"}),
		thought_signatures: None,
	})]));
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
		ContentPart::from_text("thanks, and tomorrow?"),
	]);
	let chat_req = ChatRequest::new(vec![assistant_msg, user_msg]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 3, "assistant + extracted tool + user");
	assert_eq!(messages[0]["role"], "assistant");
	assert_eq!(
		messages[1],
		json!({"role": "tool", "content": "sunny", "tool_call_id": "call_1"}),
		"embedded tool response must become a role:\"tool\" message before the user message"
	);
	assert_eq!(
		messages[2],
		json!({"role": "user", "content": [{"type": "text", "text": "thanks, and tomorrow?"}]})
	);

	Ok(())
}

/// Image parts of a user-embedded `ToolResponse` fold into the SAME user message
/// (as `image_url` blocks), mirroring the Gemini serializer's user-embedded
/// handling, while the extracted tool message keeps the text.
#[test]
fn test_user_embedded_tool_response_image_part_folds_into_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response =
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(tool_response),
		ContentPart::from_text("what do you see?"),
	]);
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 2, "extracted tool + user (no separate image message)");
	assert_eq!(
		messages[0],
		json!({"role": "tool", "content": "screenshot taken", "tool_call_id": "call_1"})
	);
	assert_eq!(
		messages[1],
		json!({
			"role": "user",
			"content": [
				{"type": "image_url", "image_url": {"url": "data:image/png;base64,PNG64"}},
				{"type": "text", "text": "what do you see?"},
			]
		}),
		"the rescued image must ride in the same user message, without a label message"
	);

	Ok(())
}

/// A user message whose content is ONLY embedded tool responses leaves nothing to
/// carry: the tool messages are extracted (multiple, in order) and the now-empty
/// user message is omitted. `call_id`s are serialized as-is (no matching validation).
#[test]
fn test_user_message_with_only_embedded_tool_responses_omits_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "42")),
		ContentPart::ToolResponse(ToolResponse::new("call_unmatched", "43")),
	]);
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 2, "only the two extracted tool messages");
	assert_eq!(
		messages[0],
		json!({"role": "tool", "content": "42", "tool_call_id": "call_1"})
	);
	assert_eq!(
		messages[1],
		json!({"role": "tool", "content": "43", "tool_call_id": "call_unmatched"})
	);

	Ok(())
}

/// When a user message with an embedded `ToolResponse` follows a Tool-role run
/// whose images are pending, the extracted tool message must land BEFORE the
/// batched tool-images user message, keeping it adjacent to the tool-message run
/// (the wire rejects a tool message that follows a user message).
#[test]
fn test_user_embedded_tool_response_stays_adjacent_to_tool_run() -> Result<()> {
	// -- Setup & Fixtures
	let tool_msg = ChatMessage::from(
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]),
	);
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_2", "done")),
		ContentPart::from_text("go on"),
	]);
	let chat_req = ChatRequest::new(vec![tool_msg, user_msg]);

	// -- Exec
	let web_req = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages should be an array")?;
	assert_eq!(messages.len(), 4, "tool + extracted tool + image flush + user");
	assert_eq!(messages[0]["role"], "tool");
	assert_eq!(messages[0]["tool_call_id"], "call_1");
	assert_eq!(
		messages[1],
		json!({"role": "tool", "content": "done", "tool_call_id": "call_2"}),
		"extracted tool message must come before the batched tool-images user message"
	);
	assert_eq!(messages[2]["content"][0]["text"], "Attached image(s) from tool result:");
	assert_eq!(
		messages[3],
		json!({"role": "user", "content": [{"type": "text", "text": "go on"}]})
	);

	Ok(())
}

/// A `ToolResponse` embedded in an Assistant message has no representation on any
/// provider wire (there is no "tool result authored by the assistant"), so the
/// serializer must reject the shape with a hard error instead of dropping the
/// content or inventing a placement.
#[test]
fn test_assistant_embedded_tool_response_is_rejected() -> Result<()> {
	// -- Setup & Fixtures
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![
		ContentPart::from_text("checking"),
		ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({"city": "Paris"}),
			thought_signatures: None,
		}),
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
	]));
	let chat_req = ChatRequest::new(vec![ChatMessage::user("weather?"), assistant_msg]);

	// -- Exec
	let err = OpenAIAdapter::util_to_web_request_data(
		target("gpt-4o-mini"),
		crate::adapter::ServiceType::Chat,
		chat_req,
		ChatOptionsSet::default(),
		None,
	)
	.expect_err("assistant-embedded tool response must fail serialization");

	// -- Check
	let crate::Error::MessageContentTypeNotSupported { cause, .. } = err else {
		return Err(format!("expected MessageContentTypeNotSupported, got: {err}").into());
	};
	assert!(
		cause.contains("Assistant-role message"),
		"cause must name the unsupported shape: {cause}"
	);
	assert!(
		cause.contains("Tool-role message"),
		"cause must point at the supported Tool-role shape: {cause}"
	);

	Ok(())
}

// endregion: --- Embedded Tool Responses

// region:    --- Support

fn test_model() -> ModelIden {
	ModelIden::new(AdapterKind::OpenAI, "test-model")
}

fn target(model_name: &str) -> ServiceTarget {
	ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAI, model_name),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://api.openai.com/v1/"),
	}
}

fn managed_options() -> ToWebRequestDataOptions {
	ToWebRequestDataOptions {
		managed_body_thinking: true,
		..Default::default()
	}
}

fn payload(
	model_name: &str,
	options_set: ChatOptionsSet<'_, '_>,
	custom: Option<ToWebRequestDataOptions>,
) -> Result<Value> {
	Ok(OpenAIAdapter::util_to_web_request_data(
		target(model_name),
		crate::adapter::ServiceType::Chat,
		ChatRequest::from_user("hello"),
		options_set,
		custom,
	)?
	.payload)
}

// endregion: --- Support
