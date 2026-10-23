use super::*;
use crate::adapter::AdapterKind;
use crate::chat::{ChatMessage, ChatOptions, JsonSpec, Tool, ToolCall, ToolChoice, ToolResponse};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_openai_resp_cache_control_without_eligible_content_does_not_fail_response_request() {
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

#[test]
fn function_tool_strict_is_sent_only_when_set() -> Result<()> {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let schema = json!({
		"type": "object",
		"properties": {"city": {"type": "string"}}
	});
	let request = ChatRequest::from_user("weather").with_tools(vec![
		Tool::new("unset").with_schema(schema.clone()),
		Tool::new("not_strict").with_schema(schema.clone()).with_strict(false),
		Tool::new("strict").with_schema(schema.clone()).with_strict(true),
	]);

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())?;
	let tools = &web_req.payload["tools"];

	assert!(tools[0].get("strict").is_none());
	assert_eq!(tools[0]["parameters"], schema);
	assert_eq!(tools[1]["strict"], false);
	assert_eq!(tools[1]["parameters"], schema);
	assert_eq!(tools[2]["strict"], true);
	assert_eq!(tools[2]["parameters"]["additionalProperties"], false);

	Ok(())
}

#[test]
fn test_openai_resp_extra_body_merged_into_response_payload() {
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
fn test_openai_resp_tool_choice_specific_tool_serialized_on_response_payload() {
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
fn test_openai_resp_assistant_message_uses_output_text_content_type() {
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
fn test_openai_resp_gpt_5_6_responses_defaults_to_explicit_cache_mode() {
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
fn test_openai_resp_gpt_6_responses_defaults_to_explicit_cache_mode() {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-6"),
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
fn test_openai_resp_gpt_5_6_codex_responses_endpoint_omits_prompt_cache_options() -> Result<()> {
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
fn test_openai_resp_gpt_5_6_responses_cache_key_uses_api_default_cache_mode() {
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
fn test_openai_resp_gpt_5_6_responses_places_breakpoint_on_last_eligible_content_block() {
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
fn test_openai_resp_gpt_5_6_responses_ignores_tool_cache_control() {
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
fn test_openai_resp_gpt_5_5_responses_keeps_legacy_cache_retention() {
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

#[test]
fn test_openai_resp_adapter_adapters_openai_resp_tool_to_openai_tool_optional_description() -> Result<()> {
	// -- Setup & Fixtures
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let schema = json!({
		"type": "object",
		"properties": {"city": {"type": "string"}}
	});
	let request = ChatRequest::from_user("weather").with_tools(vec![
		Tool::new("without_description").with_schema(schema.clone()),
		Tool::new("with_description")
			.with_schema(schema.clone())
			.with_description("Get the weather"),
	]);

	// -- Exec
	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())?;

	// -- Check
	let tools = web_req.payload["tools"].as_array().ok_or("Should have a tools array")?;
	assert_eq!(tools.len(), 2);
	assert_eq!(tools[0]["type"], "function");
	assert_eq!(tools[0]["name"], "without_description");
	assert_eq!(tools[0]["parameters"], schema);
	assert!(tools[0].get("description").is_none());
	assert_eq!(tools[1]["type"], "function");
	assert_eq!(tools[1]["description"], "Get the weather");
	assert_eq!(tools[1]["parameters"], schema);

	Ok(())
}

#[test]
fn test_openai_resp_to_chat_response_captures_reasoning_and_thought_signature() -> Result<()> {
	let model_iden = ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.4-mini");
	let web_response = WebResponse {
		status: reqwest::StatusCode::OK,
		body: json!({
			"id": "resp_123",
			"status": "completed",
			"model": "gpt-5.4-mini-2026-03-17",
			"output": [
				{
					"id": "rs_1",
					"type": "reasoning",
					"encrypted_content": "enc_sig_1",
					"summary": [
						{"type": "summary_text", "text": "**Checking weather**\nLooking up Paris."}
					],
					"content": [
						{"type": "reasoning_text", "text": "Need to call get_weather."}
					]
				},
				{
					"type": "function_call",
					"call_id": "call_1",
					"name": "get_weather",
					"arguments": "{\"city\":\"Paris\"}"
				}
			],
			"usage": {
				"input_tokens": 20,
				"output_tokens": 15,
				"total_tokens": 35
			}
		}),
	};

	let response = OpenAIRespAdapter::to_chat_response(model_iden, web_response, ChatOptionsSet::default())?;

	assert_eq!(
		response.reasoning_content.as_deref(),
		Some("**Checking weather**\nLooking up Paris.\nNeed to call get_weather.")
	);
	assert_eq!(response.content.thought_signatures(), vec!["enc_sig_1"]);

	let tool_calls = response.tool_calls();
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].call_id, "call_1");
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(
		tool_calls[0].thought_signatures.as_deref(),
		Some(["enc_sig_1".to_string()].as_slice())
	);

	Ok(())
}

#[test]
fn test_openai_resp_into_openai_request_parts_does_not_duplicate_mirrored_thought_signatures() -> Result<()> {
	let target = ServiceTarget {
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5.4-mini"),
		auth: AuthData::from_single("test-key"),
		endpoint: OpenAIRespAdapter::default_endpoint(AdapterKind::OpenAIResp),
	};
	let assistant_content = MessageContent::from_parts(vec![
		ContentPart::ThoughtSignature("enc_sig_1".to_string()),
		ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({"city": "Paris"}),
			thought_signatures: Some(vec!["enc_sig_1".to_string()]),
		}),
	]);
	let request = ChatRequest::new(vec![
		ChatMessage::user("weather in Paris"),
		ChatMessage::assistant(assistant_content),
		ChatMessage::from(ToolResponse::new("call_1", "22C")),
	]);

	let web_req =
		OpenAIRespAdapter::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())?;
	let input = web_req.payload["input"].as_array().ok_or("input should be an array")?;
	let reasoning_items: Vec<&Value> = input.iter().filter(|item| item["type"] == "reasoning").collect();

	assert_eq!(
		reasoning_items.len(),
		1,
		"encrypted reasoning blob must not be emitted twice"
	);
	assert_eq!(reasoning_items[0]["encrypted_content"], "enc_sig_1");

	Ok(())
}
