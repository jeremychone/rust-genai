type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

use super::*;
use crate::adapter::AdapterKind;
use crate::chat::{Binary, ChatMessage, ChatOptions, JsonSpec, Tool, ToolCall, ToolChoice};

#[test]
fn test_cache_control_without_eligible_content_does_not_fail_response_request() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
		call_id: "call_1".to_string(),
		fn_name: "get_weather".to_string(),
		fn_arguments: json!({}),
		thought_signatures: None,
	})]))
	.with_options(CacheControl::Ephemeral);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("hello"), assistant_msg]);

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("unsupported breakpoint placement should be ignored");

	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");
}

#[test]
fn custom_grammar_tool_and_roundtrip_use_responses_native_items() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let patch = "*** Begin Patch\n*** Update File: source.c\n@@\n-old\n+new\n*** End Patch\n";
	let assistant = ChatMessage::assistant(vec![ToolCall {
		call_id: "call_patch".to_string(),
		fn_name: "apply_patch".to_string(),
		fn_arguments: Value::String(patch.to_string()),
		thought_signatures: None,
	}]);
	let response = ChatMessage::from(ToolResponse::new("call_patch", "Done!"));
	let format = json!({
		"type": "grammar",
		"syntax": "lark",
		"definition": "start: PATCH",
	});
	let request = ChatRequest::new(vec![ChatMessage::user("patch it"), assistant, response]).with_tools(vec![
		Tool::new("apply_patch")
			.with_description("Apply a patch")
			.with_custom_format(format.clone()),
	]);

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default()).unwrap();
	assert_eq!(
		web_req.payload["tools"][0],
		json!({
			"type": "custom",
			"name": "apply_patch",
			"description": "Apply a patch",
			"format": format,
		})
	);
	let input = web_req.payload["input"].as_array().unwrap();
	assert!(input.iter().any(|item| {
		item["type"] == "custom_tool_call" && item["call_id"] == "call_patch" && item["input"] == patch
	}));
	assert!(input.iter().any(|item| {
		item["type"] == "custom_tool_call_output" && item["call_id"] == "call_patch" && item["output"] == "Done!"
	}));
}

/// A `ToolResponse` with an image part must serialize natively as a
/// `function_call_output` whose `output` is an array of `input_text` / `input_image`
/// items (the Responses API supports image function-call outputs natively).
#[test]
fn test_tool_response_image_part_serializes_native_output_array() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let tool_response =
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	let item = input
		.iter()
		.find(|item| item["type"] == "function_call_output")
		.expect("function_call_output item must be present");
	assert_eq!(item["call_id"], "call_1");
	assert_eq!(
		item["output"],
		json!([
			{"type": "input_text", "text": "screenshot taken"},
			{"type": "input_image", "detail": "auto", "image_url": "data:image/png;base64,PNG64"},
		])
	);
}

/// Regression guard: a text-only `ToolResponse` must keep `output` as a plain string.
#[test]
fn test_tool_response_text_only_output_stays_string() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	let item = input
		.iter()
		.find(|item| item["type"] == "function_call_output")
		.expect("function_call_output item must be present");
	assert_eq!(
		item["output"],
		json!("42"),
		"text-only output must remain a plain string"
	);
}

/// A `ToolResponse` whose `call_id` belongs to a CUSTOM tool serializes as a
/// `custom_tool_call_output` with a raw string `output` (placeholder text when the
/// result is image-only), and its image parts are rescued into a follow-up `user`
/// message input item right after the output item.
#[test]
fn test_custom_tool_response_image_part_rides_in_followup_user_item() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant = ChatMessage::assistant(vec![ToolCall {
		call_id: "call_patch".to_string(),
		fn_name: "apply_patch".to_string(),
		fn_arguments: Value::String("some patch".to_string()),
		thought_signatures: None,
	}]);
	let response = ChatMessage::from(ToolResponse::new("call_patch", "").with_parts([Binary::from_base64(
		"image/png",
		"PNG64",
		None,
	)]));
	let request = ChatRequest::new(vec![ChatMessage::user("patch it"), assistant, response]).with_tools(vec![
		Tool::new("apply_patch").with_custom_format(json!({"type": "text"})),
	]);

	// -- Exec
	let web_req = OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())
		.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	let output_idx = input
		.iter()
		.position(|item| item["type"] == "custom_tool_call_output")
		.expect("custom_tool_call_output item must be present");
	assert_eq!(input[output_idx]["call_id"], "call_patch");
	assert_eq!(
		input[output_idx]["output"],
		json!("(see attached image)"),
		"custom output must stay a raw string with the image placeholder"
	);
	let followup = input
		.get(output_idx + 1)
		.expect("follow-up user message item must come right after the custom output");
	assert_eq!(
		*followup,
		json!({
			"type": "message",
			"role": "user",
			"content": [
				{"type": "input_text", "text": TOOL_RESULT_IMAGES_LABEL},
				{"type": "input_image", "detail": "auto", "image_url": "data:image/png;base64,PNG64"},
			]
		})
	);
}

/// Regression guard: a text-only custom tool output keeps its raw-string `output`
/// and does NOT get a follow-up user message item.
#[test]
fn test_custom_tool_response_text_only_has_no_followup_item() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant = ChatMessage::assistant(vec![ToolCall {
		call_id: "call_patch".to_string(),
		fn_name: "apply_patch".to_string(),
		fn_arguments: Value::String("some patch".to_string()),
		thought_signatures: None,
	}]);
	let response = ChatMessage::from(ToolResponse::new("call_patch", "Done!"));
	let request = ChatRequest::new(vec![ChatMessage::user("patch it"), assistant, response]).with_tools(vec![
		Tool::new("apply_patch").with_custom_format(json!({"type": "text"})),
	]);

	// -- Exec
	let web_req = OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())
		.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	let item = input
		.iter()
		.find(|item| item["type"] == "custom_tool_call_output")
		.expect("custom_tool_call_output item must be present");
	assert_eq!(
		item["output"],
		json!("Done!"),
		"text-only output must remain the raw string"
	);
	assert!(
		!input.iter().any(|item| item["content"][0]["text"] == TOOL_RESULT_IMAGES_LABEL),
		"no follow-up tool-images user item must be emitted for a text-only custom output"
	);
}

#[test]
fn test_extra_body_merged_into_response_payload() {
	let chat_options = ChatOptions::default()
		.with_top_p(0.3)
		.with_extra_body(json!({"top_p": 0.9, "metadata": {"source": "test"}}));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["top_p"], 0.9);
	assert_eq!(web_req.payload["metadata"]["source"], "test");
}

#[test]
fn pydantic_union_schema_is_sanitized_for_responses() {
	let schema = json!({
		"type": "object",
		"properties": {
			"animal": {
				"discriminator": {"propertyName": "kind"},
				"oneOf": [{"$ref": "#/$defs/Cat"}, {"$ref": "#/$defs/Dog"}]
			}
		},
		"$defs": {
			"Cat": {
				"type": "object",
				"properties": {"kind": {"const": "cat"}},
				"required": ["kind"]
			},
			"Dog": {
				"type": "object",
				"properties": {"kind": {"const": "dog"}},
				"required": ["kind"]
			}
		}
	});
	let options = ChatOptions::default().with_response_format(JsonSpec::new("union", schema));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};

	let web_req = OpenAIRespAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		ChatRequest::from_user("return an animal"),
		options_set,
	)
	.unwrap();

	let animal = &web_req.payload["text"]["format"]["schema"]["properties"]["animal"];
	assert!(animal.get("oneOf").is_none());
	assert_eq!(animal["discriminator"], json!({"propertyName": "kind"}));
	assert_eq!(
		animal["anyOf"],
		json!([{"$ref": "#/$defs/Cat"}, {"$ref": "#/$defs/Dog"}])
	);
}

#[test]
fn dynamic_map_schema_is_sent_to_backend_for_validation() {
	let schema = json!({
		"type": "object",
		"properties": {
			"lookup": {"type": "object", "additionalProperties": {"type": "integer"}}
		}
	});
	let options = ChatOptions::default().with_response_format(JsonSpec::new("mapping", schema));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};

	let web_req = OpenAIRespAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		ChatRequest::from_user("return a mapping"),
		options_set,
	)
	.unwrap();

	assert_eq!(
		web_req.payload["text"]["format"]["schema"]["properties"]["lookup"]["additionalProperties"],
		json!({"type": "integer"})
	);
}

#[test]
fn test_tool_choice_specific_tool_serialized_on_response_payload() {
	let chat_options = ChatOptions::default().with_tool_choice(ToolChoice::tool("get_weather"));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_req = ChatRequest::from_user("weather").with_tools(vec![Tool::new("get_weather")]);

	let web_req = OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	assert_eq!(
		web_req.payload["tool_choice"],
		json!({
			"type": "function",
			"name": "get_weather"
		})
	);
}

/// Test that assistant message text content uses "output_text" type (not "input_text").
///
/// This is required by OpenAI's Responses API - assistant content is model output,
/// so it must use "output_text". Using "input_text" causes:
/// "Invalid value: 'input_text'. Supported values are: 'output_text' and 'refusal'."
#[test]
fn test_assistant_message_uses_output_text_content_type() {
	let model_iden = ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-codex");

	// Create a chat request with an assistant message
	let chat_req = ChatRequest::default()
		.with_system("You are a helpful assistant.")
		.append_message(ChatMessage::user("What's the weather?"))
		.append_message(ChatMessage::assistant("The weather is sunny."));

	// Serialize to OpenAI Responses API format
	let parts = OpenAIRespAdapter::into_openai_request_parts(&model_iden, chat_req, None)
		.expect("Should serialize successfully");

	// Find the assistant message in input_items
	let assistant_msg = parts
		.input_items
		.iter()
		.find(|item| {
			item.get("type").and_then(|t| t.as_str()) == Some("message")
				&& item.get("role").and_then(|r| r.as_str()) == Some("assistant")
		})
		.expect("Should have an assistant message");

	// Check the content uses "output_text" type
	let content = assistant_msg
		.get("content")
		.and_then(|c| c.as_array())
		.expect("Assistant message should have content array");

	assert!(!content.is_empty(), "Content should not be empty");

	let first_content = &content[0];
	let content_type = first_content
		.get("type")
		.and_then(|t| t.as_str())
		.expect("Content should have a type");

	assert_eq!(
		content_type, "output_text",
		"Assistant message content should use 'output_text' type, not 'input_text'"
	);
}

#[test]
fn test_gpt_5_6_responses_defaults_to_explicit_cache_mode() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};

	let web_req = OpenAIRespAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		ChatOptionsSet::default(),
	)
	.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");
	assert!(web_req.payload["prompt_cache_options"].get("ttl").is_none());
	assert!(
		web_req.payload["input"][0]["content"][0]
			.get("prompt_cache_breakpoint")
			.is_none()
	);
}

#[test]
fn test_gpt_5_6_codex_responses_endpoint_omits_prompt_cache_options() -> Result<()> {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: Endpoint::from_static("https://chatgpt.com/backend-api/codex/"),
	};

	let web_req = OpenAIRespAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		ChatRequest::from_user("hello"),
		ChatOptionsSet::default(),
	)?;

	assert!(web_req.payload.get("prompt_cache_options").is_none());
	Ok(())
}

#[test]
fn test_gpt_5_6_responses_cache_key_uses_api_default_cache_mode() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_options = ChatOptions::default().with_prompt_cache_key("stable-key");
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert!(web_req.payload.get("prompt_cache_options").is_none());
	assert!(
		web_req.payload["input"][0]["content"][0]
			.get("prompt_cache_breakpoint")
			.is_none()
	);
}

#[test]
fn test_gpt_5_6_responses_places_breakpoint_on_last_eligible_content_block() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_req = ChatRequest::new(vec![
		ChatMessage::user(vec![
			ContentPart::from_text("stable text"),
			ContentPart::from_binary_url("image/png", "https://example.com/image.png", None),
			ContentPart::from_text("last text"),
		])
		.with_options(CacheControl::Ephemeral),
	]);

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	let blocks = web_req.payload["input"][0]["content"]
		.as_array()
		.expect("message content should be an array");
	assert!(blocks[0].get("prompt_cache_breakpoint").is_none());
	assert!(blocks[1].get("prompt_cache_breakpoint").is_none());
	assert_eq!(blocks[2]["prompt_cache_breakpoint"]["mode"], "explicit");
}

#[test]
fn test_gpt_5_6_responses_ignores_tool_cache_control() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_req = ChatRequest::from_user("hello")
		.append_tool(Tool::new("get_weather").with_cache_control(CacheControl::Ephemeral));

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("tool cache control should be ignored");

	assert_eq!(web_req.payload["prompt_cache_options"]["mode"], "explicit");
	assert!(web_req.payload["tools"][0].get("prompt_cache_breakpoint").is_none());
}

#[test]
fn test_gpt_5_5_responses_keeps_legacy_cache_retention() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.5"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let chat_options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral24h);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)
			.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["prompt_cache_retention"], "24h");
	assert!(web_req.payload.get("prompt_cache_options").is_none());
}

// region:    --- Embedded Tool Responses

/// A `ToolResponse` embedded in a User-role message (Anthropic-style user-carried
/// tool result) must be extracted as a `function_call_output` item placed BEFORE
/// the user message item carrying the remaining content. `call_id`s are serialized
/// as-is (no matching validation; here the call_id matches no `function_call` item).
#[test]
fn test_user_embedded_tool_response_extracted_before_user_item() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
		ContentPart::from_text("thanks, and tomorrow?"),
	]);
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	assert_eq!(input.len(), 2, "extracted output item + user message item");
	assert_eq!(
		input[0],
		json!({"type": "function_call_output", "call_id": "call_1", "output": "sunny"}),
		"embedded tool response must become a function_call_output item before the user message item"
	);
	assert_eq!(
		input[1],
		json!({"role": "user", "content": [{"type": "input_text", "text": "thanks, and tomorrow?"}]})
	);
}

/// Image parts of a user-embedded `ToolResponse` for a FUNCTION tool ride natively
/// in the `function_call_output` `output` array (`input_text` + `input_image`);
/// nothing is folded into the carrying user message item.
#[test]
fn test_user_embedded_tool_response_image_part_uses_native_output_array() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let tool_response =
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(tool_response),
		ContentPart::from_text("what do you see?"),
	]);
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	assert_eq!(input.len(), 2, "extracted output item + user message item");
	assert_eq!(
		input[0],
		json!({
			"type": "function_call_output",
			"call_id": "call_1",
			"output": [
				{"type": "input_text", "text": "screenshot taken"},
				{"type": "input_image", "detail": "auto", "image_url": "data:image/png;base64,PNG64"},
			]
		})
	);
	assert_eq!(
		input[1],
		json!({"role": "user", "content": [{"type": "input_text", "text": "what do you see?"}]}),
		"function-output images ride natively; the user message item must not carry them"
	);
}

/// Image parts of a user-embedded `ToolResponse` for a CUSTOM tool (raw-string
/// output wire) fold into the SAME user message item as `input_image` items (no
/// label item), mirroring the Gemini serializer's user-embedded handling.
#[test]
fn test_user_embedded_custom_tool_response_images_fold_into_user_item() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant = ChatMessage::assistant(vec![ToolCall {
		call_id: "call_patch".to_string(),
		fn_name: "apply_patch".to_string(),
		fn_arguments: Value::String("some patch".to_string()),
		thought_signatures: None,
	}]);
	let tool_response =
		ToolResponse::new("call_patch", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(tool_response),
		ContentPart::from_text("continue"),
	]);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("patch it"), assistant, user_msg]).with_tools(vec![
		Tool::new("apply_patch").with_custom_format(json!({"type": "text"})),
	]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	let output_idx = input
		.iter()
		.position(|item| item["type"] == "custom_tool_call_output")
		.expect("custom_tool_call_output item must be present");
	assert_eq!(
		input[output_idx],
		json!({"type": "custom_tool_call_output", "call_id": "call_patch", "output": "(see attached image)"})
	);
	assert_eq!(
		input[output_idx + 1],
		json!({
			"role": "user",
			"content": [
				{"type": "input_image", "detail": "auto", "image_url": "data:image/png;base64,PNG64"},
				{"type": "input_text", "text": "continue"},
			]
		}),
		"the rescued image must fold into the same user message item"
	);
	assert!(
		!input.iter().any(|item| item["content"][0]["text"] == TOOL_RESULT_IMAGES_LABEL),
		"no separate labeled tool-images user item must be emitted"
	);
}

/// A user message whose content is ONLY embedded tool responses leaves nothing to
/// carry: the output items are extracted (multiple, in order) and the now-empty
/// user message item is omitted.
#[test]
fn test_user_item_with_only_embedded_tool_responses_is_omitted() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "42")),
		ContentPart::ToolResponse(ToolResponse::new("call_2", "43")),
	]);
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	assert_eq!(input.len(), 2, "only the two extracted output items");
	assert_eq!(
		input[0],
		json!({"type": "function_call_output", "call_id": "call_1", "output": "42"})
	);
	assert_eq!(
		input[1],
		json!({"type": "function_call_output", "call_id": "call_2", "output": "43"})
	);
	assert!(
		!input.iter().any(|item| item["role"] == "user"),
		"no empty user message item must be emitted"
	);
}

/// When a user message with an embedded `ToolResponse` follows a Tool-role run
/// whose custom-output images are pending, the extracted output item must land
/// BEFORE the batched tool-images user item, keeping it adjacent to the tool run
/// (mirroring the Chat Completions adjacency behavior).
#[test]
fn test_user_embedded_tool_response_stays_adjacent_to_tool_run() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant = ChatMessage::assistant(vec![ToolCall {
		call_id: "call_patch".to_string(),
		fn_name: "apply_patch".to_string(),
		fn_arguments: Value::String("some patch".to_string()),
		thought_signatures: None,
	}]);
	let tool_msg = ChatMessage::from(ToolResponse::new("call_patch", "").with_parts([Binary::from_base64(
		"image/png",
		"PNG64",
		None,
	)]));
	let user_msg = ChatMessage::user(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_2", "done")),
		ContentPart::from_text("go on"),
	]);
	let chat_req = ChatRequest::new(vec![assistant, tool_msg, user_msg]).with_tools(vec![
		Tool::new("apply_patch").with_custom_format(json!({"type": "text"})),
	]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
			.expect("to_web_request_data should succeed");

	// -- Check
	let input = web_req.payload["input"].as_array().expect("input array");
	assert_eq!(
		input.len(),
		5,
		"call + custom output + extracted output + image flush + user"
	);
	assert_eq!(input[0]["type"], "custom_tool_call");
	assert_eq!(input[1]["type"], "custom_tool_call_output");
	assert_eq!(
		input[2],
		json!({"type": "function_call_output", "call_id": "call_2", "output": "done"}),
		"extracted output item must come before the batched tool-images user item"
	);
	assert_eq!(input[3]["content"][0]["text"], TOOL_RESULT_IMAGES_LABEL);
	assert_eq!(
		input[4],
		json!({"role": "user", "content": [{"type": "input_text", "text": "go on"}]})
	);
}

/// A `ToolResponse` embedded in an Assistant message has no representation on any
/// provider wire (there is no "tool result authored by the assistant"), so the
/// serializer must reject the shape with a hard error instead of dropping the
/// content or inventing a placement.
#[test]
fn test_assistant_embedded_tool_response_is_rejected() {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.6"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
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
	let err = OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
		.expect_err("assistant-embedded tool response must fail serialization");

	// -- Check
	let Error::MessageContentTypeNotSupported { cause, .. } = err else {
		panic!("expected MessageContentTypeNotSupported, got: {err}");
	};
	assert!(
		cause.contains("Assistant-role message"),
		"cause must name the unsupported shape: {cause}"
	);
	assert!(
		cause.contains("Tool-role message"),
		"cause must point at the supported Tool-role shape: {cause}"
	);
}

// endregion: --- Embedded Tool Responses
