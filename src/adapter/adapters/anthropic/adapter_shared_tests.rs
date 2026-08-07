type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::ServiceTarget;
use crate::adapter::adapters::anthropic::ant_reasoning::REASONING_HIGH;
use crate::adapter::{Adapter, ServiceType};
use crate::chat::{ChatMessage, ChatOptions, ChatRequest, JsonSpec, Tool, ToolChoice};
use crate::resolver::AuthData;

/// Regression guard: when both `reasoning_effort` and `JsonSpec` response format are set
/// on a model that uses the `output_config` effort API (e.g. `claude-sonnet-4-6`), both
/// `effort` and `format` must appear inside the same `output_config` JSON object.
#[test]
fn test_anthropic_output_config_merges_effort_and_format() {
	let chat_options = ChatOptions {
		reasoning_effort: Some(ReasoningEffort::High),
		response_format: Some(ChatResponseFormat::JsonSpec(JsonSpec::new(
			"anthropic_ignores_name", // NOTE: Anthropic doesn't recognize a "name" field
			json!({"type": "object", "properties": {}}),
		))),
		..Default::default()
	};

	let model_iden = ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6");
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: model_iden,
	};
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));

	let result =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set);

	let web_req = result.expect("to_web_request_data should succeed");
	let output_config = web_req.payload.get("output_config").expect("output_config must be present");

	assert_eq!(
		output_config.get("effort").and_then(|v| v.as_str()),
		Some("high"),
		"effort must be present in output_config"
	);
	assert_eq!(
		output_config.get("format").and_then(|f| f.get("type")).and_then(|v| v.as_str()),
		Some("json_schema"),
		"format.type must be present in output_config"
	);
	assert_eq!(output_config["format"]["schema"]["additionalProperties"], json!(false));
}

#[test]
fn test_anthropic_dynamic_map_response_schema_is_sent_for_backend_validation() {
	let chat_options = ChatOptions::default().with_response_format(JsonSpec::new(
		"mapping",
		json!({
			"type": "object",
			"properties": {
				"lookup": {"type": "object", "additionalProperties": {"type": "integer"}}
			}
		}),
	));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		ChatRequest::from_user("return a mapping"),
		ChatOptionsSet::default().with_chat_options(Some(&chat_options)),
	)
	.unwrap();

	assert_eq!(
		web_req.payload["output_config"]["format"]["schema"]["properties"]["lookup"]["additionalProperties"],
		json!({"type": "integer"})
	);
}

#[test]
fn test_anthropic_anthropic_tool_eager_input_streaming_serializes() {
	// with_eager_input_streaming(true) → the tool carries `eager_input_streaming: true`
	// (Anthropic fine-grained tool streaming, GA — opt-in per tool).
	let tool = Tool::new("manage_calendar")
		.with_schema(json!({"type": "object", "properties": {}}))
		.with_eager_input_streaming(true);
	let req = ChatRequest::from_user("hi").with_tools(vec![tool]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(
		target,
		ServiceType::Chat,
		req,
		ChatOptionsSet::default().with_chat_options(None),
	)
	.expect("to_web_request_data should succeed");

	let tools = web_req.payload.get("tools").and_then(|t| t.as_array()).expect("tools array");
	assert_eq!(
		tools[0].get("eager_input_streaming").and_then(|v| v.as_bool()),
		Some(true),
		"eager_input_streaming must serialize onto the tool"
	);
}

#[test]
fn test_anthropic_non_strict_tool_schema_is_untouched() {
	let schema = json!({
		"type": "object",
		"properties": {"optional_limit": {"type": "integer", "default": 10}}
	});
	let req = ChatRequest::from_user("hi").with_tools(vec![Tool::new("search").with_schema(schema.clone())]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, req, ChatOptionsSet::default()).unwrap();

	assert_eq!(web_req.payload["tools"][0]["input_schema"], schema);
	assert!(web_req.payload["tools"][0].get("strict").is_none());
}

#[test]
fn test_anthropic_strict_tool_uses_anthropic_schema_dialect() {
	let req = ChatRequest::from_user("hi").with_tools(vec![
		Tool::new("search")
			.with_schema(json!({
				"type": "object",
				"properties": {
					"query": {"type": "string"},
					"optional_limit": {"type": "integer", "default": 10}
				},
				"required": ["query"]
			}))
			.with_strict(true),
	]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6"),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, req, ChatOptionsSet::default()).unwrap();
	let tool = &web_req.payload["tools"][0];

	assert_eq!(tool["strict"], json!(true));
	assert_eq!(tool["input_schema"]["required"], json!(["query"]));
	assert_eq!(
		tool["input_schema"]["properties"]["optional_limit"]["default"],
		json!(10)
	);
	assert_eq!(tool["input_schema"]["additionalProperties"], json!(false));
}

#[test]
fn test_anthropic_tool_choice_required_serialized_on_anthropic_payload() {
	let chat_options = ChatOptions::default().with_tool_choice(ToolChoice::Required);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-sonnet-4-6"),
	};
	let chat_req = ChatRequest::from_user("weather").with_tools(vec![Tool::new("get_weather")]);

	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	assert_eq!(web_req.payload["tool_choice"], json!({"type": "any"}));
}

/// A `cache_control` set on a `Tool` must be serialized onto that tool in the
/// Anthropic `tools` payload, so the tool-definition prefix can be prompt-cached.
#[test]
fn test_anthropic_tool_cache_control_serialized_on_anthropic_payload() {
	// -- Setup & Fixtures
	let ephemeral_tool = Tool::new("get_weather")
		.with_description("Get the current weather for a location")
		.with_schema(json!({"type": "object", "properties": {}}))
		.with_cache_control(CacheControl::Ephemeral);
	let one_hour_tool = Tool::new("get_time")
		.with_description("Get the current time for a location")
		.with_schema(json!({"type": "object", "properties": {}}))
		.with_cache_control(CacheControl::Ephemeral1h);
	let plain_tool = Tool::new("get_news")
		.with_description("Get the latest news")
		.with_schema(json!({"type": "object", "properties": {}}));

	let chat_req = ChatRequest::from_user("hello").with_tools([ephemeral_tool, one_hour_tool, plain_tool]);

	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};
	let options_set = ChatOptionsSet::default();

	// -- Exec
	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	// -- Check
	let tools = web_req
		.payload
		.get("tools")
		.and_then(|t| t.as_array())
		.expect("tools array must be present");
	assert_eq!(tools.len(), 3, "all three tools must be present");
	assert_eq!(
		tools[0].get("cache_control"),
		Some(&json!({"type": "ephemeral"})),
		"Ephemeral must serialize without a ttl"
	);
	assert_eq!(
		tools[1].get("cache_control"),
		Some(&json!({"type": "ephemeral", "ttl": "1h"})),
		"Ephemeral1h must serialize with ttl '1h'"
	);
	assert_eq!(
		tools[2].get("cache_control"),
		None,
		"a tool without cache_control must not emit the field"
	);
}

/// Request-level cache_control with no explicit breakpoint must auto-mark the last
/// system block (caching the tools+system prefix) on Anthropic.
#[test]
fn test_anthropic_request_level_cache_control_auto_breakpoint_on_system() {
	let chat_req = ChatRequest::from_user("hello").with_system("a long, stable system prompt");
	let chat_options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	let system = web_req.payload.get("system").expect("system must be present");
	let parts = system
		.as_array()
		.expect("system must be a multi-part array when cache_control is applied");
	let last = parts.last().expect("system must have at least one part");
	assert_eq!(
		last.get("cache_control"),
		Some(&json!({"type": "ephemeral"})),
		"request-level cache_control must auto-apply to the last system block"
	);
}

/// With no system but tools present, request-level cache_control falls back to the last tool.
#[test]
fn test_anthropic_request_level_cache_control_falls_back_to_last_tool_when_no_system() {
	let tool_a = Tool::new("a").with_schema(json!({"type": "object", "properties": {}}));
	let tool_b = Tool::new("b").with_schema(json!({"type": "object", "properties": {}}));
	let chat_req = ChatRequest::from_user("hello").with_tools([tool_a, tool_b]);
	let chat_options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	let tools = web_req.payload.get("tools").and_then(|t| t.as_array()).expect("tools array");
	assert_eq!(tools.len(), 2);
	assert_eq!(tools[0].get("cache_control"), None, "non-last tool must not be marked");
	assert_eq!(
		tools[1].get("cache_control"),
		Some(&json!({"type": "ephemeral"})),
		"request-level cache_control must fall back to the last tool"
	);
	assert!(web_req.payload.get("system").is_none(), "no system should be present");
}

/// With neither system nor tools, request-level cache_control is a no-op.
#[test]
fn test_anthropic_request_level_cache_control_noop_without_static_prefix() {
	let chat_req = ChatRequest::from_user("hello");
	let chat_options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	assert!(web_req.payload.get("system").is_none(), "no system");
	assert!(web_req.payload.get("tools").is_none(), "no tools");
	let messages = web_req.payload.get("messages").and_then(|m| m.as_array()).expect("messages");
	assert!(
		messages[0]["content"].is_string(),
		"user content must be a plain string with no cache_control breakpoint"
	);
}

/// When an explicit message-level breakpoint exists, request-level cache_control must
/// NOT auto-apply to the system prefix (defer to the explicit breakpoint).
#[test]
fn test_anthropic_request_level_cache_control_deferred_when_explicit_present() {
	let user_msg = ChatMessage::user("hello").with_options(CacheControl::Ephemeral);
	let chat_req = ChatRequest::new(vec![user_msg]).with_system("a long, stable system prompt");
	let chat_options = ChatOptions::default().with_cache_control(CacheControl::Ephemeral1h);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	let web_req = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, options_set)
		.expect("to_web_request_data should succeed");

	let system = web_req.payload.get("system").expect("system present");
	assert!(
		system.is_string(),
		"system must remain a plain string (request-level deferred to the explicit message breakpoint)"
	);
}

#[test]
fn test_anthropic_control_to_json_ephemeral() {
	let result = cache_control_to_json(&CacheControl::Ephemeral);
	assert_eq!(result, json!({"type": "ephemeral"}));
}

#[test]
fn test_anthropic_control_to_json_ephemeral_5m() {
	let result = cache_control_to_json(&CacheControl::Ephemeral5m);
	assert_eq!(result, json!({"type": "ephemeral", "ttl": "5m"}));
}

#[test]
fn test_anthropic_control_to_json_memory() {
	let result = cache_control_to_json(&CacheControl::Memory);
	assert_eq!(result, json!({"type": "ephemeral"}));
}

#[test]
fn test_anthropic_control_to_json_ephemeral_1h() {
	let result = cache_control_to_json(&CacheControl::Ephemeral1h);
	assert_eq!(result, json!({"type": "ephemeral", "ttl": "1h"}));
}

#[test]
fn test_anthropic_control_to_json_ephemeral_24h() {
	let result = cache_control_to_json(&CacheControl::Ephemeral24h);
	assert_eq!(result, json!({"type": "ephemeral", "ttl": "1h"}));
}

#[test]
fn test_anthropic_parse_cache_creation_details_with_both_ttls() {
	let cache_creation = json!({
		"ephemeral_5m_input_tokens": 456,
		"ephemeral_1h_input_tokens": 100
	});
	let result = parse_cache_creation_details(&cache_creation);
	assert!(result.is_some());
	let details = result.unwrap();
	assert_eq!(details.ephemeral_5m_tokens, Some(456));
	assert_eq!(details.ephemeral_1h_tokens, Some(100));
}

#[test]
fn test_anthropic_parse_cache_creation_details_with_5m_only() {
	let cache_creation = json!({
		"ephemeral_5m_input_tokens": 456
	});
	let result = parse_cache_creation_details(&cache_creation);
	assert!(result.is_some());
	let details = result.unwrap();
	assert_eq!(details.ephemeral_5m_tokens, Some(456));
	assert_eq!(details.ephemeral_1h_tokens, None);
}

#[test]
fn test_anthropic_parse_cache_creation_details_with_1h_only() {
	let cache_creation = json!({
		"ephemeral_1h_input_tokens": 100
	});
	let result = parse_cache_creation_details(&cache_creation);
	assert!(result.is_some());
	let details = result.unwrap();
	assert_eq!(details.ephemeral_5m_tokens, None);
	assert_eq!(details.ephemeral_1h_tokens, Some(100));
}

#[test]
fn test_anthropic_parse_cache_creation_details_empty() {
	let cache_creation = json!({});
	let result = parse_cache_creation_details(&cache_creation);
	assert!(result.is_none());
}

#[test]
fn test_anthropic_adapter_resolve_max_tokens_existing_branches() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-fable-5", MAX_TOKENS_128K),
		("claude-mythos-5", MAX_TOKENS_128K),
		("claude-sonnet-4-6", MAX_TOKENS_64K),
		("claude-haiku-4-5", MAX_TOKENS_64K),
		("claude-3-7-sonnet-latest", MAX_TOKENS_64K),
		("claude-opus-4-5", MAX_TOKENS_64K),
		("claude-opus-4-0", MAX_TOKENS_32K),
		("claude-3-5-sonnet", MAX_TOKENS_8K),
		("claude-3-opus-20240229", MAX_TOKENS_4K),
		("claude-3-haiku-20240307", MAX_TOKENS_4K),
		("unrecognized-model", MAX_TOKENS_64K),
	];
	let options_set = ChatOptionsSet::default();

	// -- Exec & Check
	for (model_name, expected) in cases {
		let capabilities = AnthropicModel::parse(model_name).capabilities();
		assert_eq!(
			AnthropicAdapter::resolve_max_tokens_for_capabilities(&capabilities, &options_set),
			expected,
			"unexpected default max_tokens for {model_name}"
		);
	}

	let chat_options = ChatOptions::default().with_max_tokens(777);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&chat_options));
	let capabilities = AnthropicModel::parse("claude-fable-5").capabilities();
	assert_eq!(
		AnthropicAdapter::resolve_max_tokens_for_capabilities(&capabilities, &options_set),
		777,
		"an explicit max_tokens value must take precedence"
	);

	Ok(())
}

#[test]
fn test_anthropic_adapter_resolve_max_tokens_preserves_broad_matching() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("custom-fable-alias", MAX_TOKENS_128K),
		("vendor-claude-sonnet-custom", MAX_TOKENS_64K),
		("vendor-claude-opus-4-custom", MAX_TOKENS_32K),
		("vendor-claude-3-5-custom", MAX_TOKENS_8K),
	];
	let options_set = ChatOptionsSet::default();

	// -- Exec & Check
	for (model_name, expected) in cases {
		let capabilities = AnthropicModel::parse(model_name).capabilities();
		assert_eq!(
			AnthropicAdapter::resolve_max_tokens_for_capabilities(&capabilities, &options_set),
			expected,
			"broad matching compatibility changed for {model_name}"
		);
	}

	Ok(())
}

#[test]
fn test_anthropic_adapter_reasoning_suffixes_are_removed() -> Result<()> {
	// -- Setup & Fixtures
	let suffixes = ["zero", "none", "minimal", "low", "medium", "high", "xhigh", "max"];

	// -- Exec & Check
	for suffix in suffixes {
		let payload = build_characterization_payload(&format!("claude-sonnet-4-6-{suffix}"), None)?;
		assert_eq!(
			payload.get("model"),
			Some(&json!("claude-sonnet-4-6")),
			"recognized suffix -{suffix} must be removed"
		);
	}

	Ok(())
}

#[test]
fn test_anthropic_adapter_namespaced_reasoning_suffix_is_removed() -> Result<()> {
	// -- Setup & Fixtures
	let model_name = "anthropic::claude-sonnet-4-6-high";

	// -- Exec
	let payload = build_characterization_payload(model_name, None)?;

	// -- Check
	assert_eq!(payload.get("model"), Some(&json!("claude-sonnet-4-6")));
	assert_eq!(payload["output_config"]["effort"], json!("high"));

	Ok(())
}

#[test]
fn test_anthropic_adapter_explicit_reasoning_precedes_suffix() -> Result<()> {
	// -- Setup & Fixtures
	let model_name = "claude-sonnet-4-6-low";

	// -- Exec
	let payload = build_characterization_payload(model_name, Some(ReasoningEffort::High))?;

	// -- Check
	assert_eq!(
		payload.get("model"),
		Some(&json!("claude-sonnet-4-6-low")),
		"the existing explicit-option path retains the model suffix"
	);
	assert_eq!(payload["output_config"]["effort"], json!("high"));

	Ok(())
}

#[test]
fn test_anthropic_adapter_alias_custom_and_unknown_reasoning_behavior() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-opus-4-6-latest", Some("high"), json!({"type": "adaptive"})),
		(
			"claude-opus-4-20250514",
			None,
			json!({"type": "enabled", "budget_tokens": REASONING_HIGH}),
		),
		(
			"custom-claude-opus-4-7-preview",
			Some("high"),
			json!({"type": "adaptive"}),
		),
		(
			"claude-opus-malformed",
			None,
			json!({"type": "enabled", "budget_tokens": REASONING_HIGH}),
		),
		(
			"unrecognized-model",
			None,
			json!({"type": "enabled", "budget_tokens": REASONING_HIGH}),
		),
	];

	// -- Exec & Check
	for (model_name, expected_effort, expected_thinking) in cases {
		let payload = build_characterization_payload(model_name, Some(ReasoningEffort::High))?;
		assert_eq!(
			payload
				.get("output_config")
				.and_then(|config| config.get("effort"))
				.and_then(Value::as_str),
			expected_effort,
			"unexpected effort behavior for {model_name}"
		);
		assert_eq!(
			payload.get("thinking"),
			Some(&expected_thinking),
			"unexpected thinking behavior for {model_name}"
		);
	}

	Ok(())
}

#[test]
fn test_anthropic_adapter_unknown_reasoning_zero_omits_fields() -> Result<()> {
	// -- Setup & Fixtures
	let model_name = "unrecognized-model";

	// -- Exec
	let payload = build_characterization_payload(model_name, Some(ReasoningEffort::Zero))?;

	// -- Check
	assert!(payload.get("thinking").is_none());
	assert!(payload.get("output_config").is_none());

	Ok(())
}

#[test]
fn test_anthropic_adapter_advanced_effort_selection() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("claude-sonnet-4-6", ReasoningEffort::Max, "max"),
		("claude-sonnet-4-6", ReasoningEffort::XHigh, "high"),
		("claude-opus-4-5", ReasoningEffort::Max, "high"),
		("claude-opus-4-5", ReasoningEffort::XHigh, "high"),
		("claude-opus-4-6", ReasoningEffort::Max, "max"),
		("claude-opus-4-6", ReasoningEffort::XHigh, "high"),
		("claude-opus-4-7", ReasoningEffort::Max, "max"),
		("claude-opus-4-7", ReasoningEffort::XHigh, "xhigh"),
		("claude-opus-4-8", ReasoningEffort::Max, "max"),
		("claude-opus-4-8", ReasoningEffort::XHigh, "xhigh"),
		("claude-opus-5", ReasoningEffort::Max, "max"),
		("claude-opus-5", ReasoningEffort::XHigh, "xhigh"),
		("claude-sonnet-5", ReasoningEffort::Max, "max"),
		("claude-sonnet-5", ReasoningEffort::XHigh, "xhigh"),
		("claude-fable-5", ReasoningEffort::Max, "max"),
		("claude-fable-5", ReasoningEffort::XHigh, "xhigh"),
		("claude-mythos-5", ReasoningEffort::Max, "max"),
		("claude-mythos-5", ReasoningEffort::XHigh, "xhigh"),
	];

	// -- Exec & Check
	for (model_name, effort, expected) in cases {
		let payload = build_characterization_payload(model_name, Some(effort))?;
		assert_eq!(
			payload["output_config"]["effort"],
			json!(expected),
			"unexpected advanced effort for {model_name}"
		);
	}

	Ok(())
}

#[test]
fn test_anthropic_adapter_opus_4_5_budget_uses_legacy_thinking() -> Result<()> {
	// -- Setup & Fixtures
	let model_name = "claude-opus-4-5";

	// -- Exec
	let payload = build_characterization_payload(model_name, Some(ReasoningEffort::Budget(1234)))?;

	// -- Check
	assert_eq!(payload["thinking"], json!({"type": "enabled", "budget_tokens": 1234}));
	assert!(payload.get("output_config").is_none());

	Ok(())
}

#[test]
fn test_anthropic_adapter_protected_reasoning_suffix_is_retained() -> Result<()> {
	// -- Setup & Fixtures
	let model_name = "deepseek-r1-zero";

	// -- Exec
	let payload = build_characterization_payload(model_name, None)?;

	// -- Check
	assert_eq!(payload.get("model"), Some(&json!(model_name)));
	assert!(payload.get("thinking").is_none());
	assert!(payload.get("output_config").is_none());

	Ok(())
}

/// A `ToolResponse` with an image part must serialize the Anthropic `tool_result`
/// content as an array of a text block followed by a base64 `image` block.
#[test]
fn test_anthropic_tool_response_image_part_serializes_tool_result_blocks() -> Result<()> {
	// -- Setup & Fixtures
	let tool_call = ToolCall {
		call_id: "call_1".to_string(),
		fn_name: "take_screenshot".to_string(),
		fn_arguments: json!({}),
		thought_signatures: None,
	};
	let tool_response = ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64(
		"image/png",
		"iVBORw0KBASE64",
		None,
	)]);
	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Take a screenshot"),
		ChatMessage::from(vec![tool_call]),
		ChatMessage::from(tool_response),
	]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	let messages = web_req.payload["messages"].as_array().ok_or("messages must be an array")?;
	assert_eq!(messages.len(), 3, "user, assistant tool_use, and tool_result messages");
	let tool_result_msg = &messages[2];
	assert_eq!(tool_result_msg["role"], json!("user"));
	assert_eq!(
		tool_result_msg["content"][0],
		json!({
			"type": "tool_result",
			"tool_use_id": "call_1",
			"content": [
				{"type": "text", "text": "screenshot taken"},
				{
					"type": "image",
					"source": {
						"type": "base64",
						"media_type": "image/png",
						"data": "iVBORw0KBASE64",
					}
				}
			]
		})
	);

	Ok(())
}

/// Regression guard: a text-only `ToolResponse` must keep the legacy `tool_result`
/// shape with a plain string `content` (no content array).
#[test]
fn test_anthropic_tool_response_text_only_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	assert_eq!(
		web_req.payload["messages"][0]["content"][0],
		json!({
			"type": "tool_result",
			"content": "42",
			"tool_use_id": "call_1",
		})
	);

	Ok(())
}

/// A `ToolResponse` embedded in an Assistant message has no representation on any
/// provider wire (there is no "tool result authored by the assistant"), so the
/// serializer must reject the shape with a hard error instead of silently dropping
/// the content (the previous behavior).
#[test]
fn test_anthropic_assistant_embedded_tool_response_is_rejected() -> Result<()> {
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
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let err = AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())
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

/// Non-image parts are not valid inside an Anthropic `tool_result`; they must be
/// skipped while the text block and image parts (including URL sources) are preserved.
/// NOTE: URL-source images were previously skipped as well; they now serialize
///       natively as `url` image sources.
#[test]
fn test_anthropic_tool_response_non_image_parts_are_skipped() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "text kept").with_parts([
		Binary::from_base64("application/pdf", "PDFDATA", Some("doc.pdf".to_string())),
		Binary::from_url("image/png", "https://example.com/shot.png", None),
	]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	assert_eq!(
		web_req.payload["messages"][0]["content"][0],
		json!({
			"type": "tool_result",
			"tool_use_id": "call_1",
			"content": [
				{"type": "text", "text": "text kept"},
				{
					"type": "image",
					"source": {
						"type": "url",
						"url": "https://example.com/shot.png",
					}
				}
			],
		}),
		"non-image parts must be skipped, keeping the text block and the URL image"
	);

	Ok(())
}

/// A `ToolResponse` with a URL-source image part must serialize the image natively
/// as an Anthropic `url` image source inside the `tool_result` content array.
#[test]
fn test_anthropic_tool_response_url_image_part_serializes_url_source() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_url(
		"image/png",
		"https://example.com/shot.png",
		None,
	)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	assert_eq!(
		web_req.payload["messages"][0]["content"][0],
		json!({
			"type": "tool_result",
			"tool_use_id": "call_1",
			"content": [
				{"type": "text", "text": "screenshot taken"},
				{
					"type": "image",
					"source": {
						"type": "url",
						"url": "https://example.com/shot.png",
					}
				}
			],
		})
	);

	Ok(())
}

/// A user message with a URL-source image must serialize natively as an Anthropic
/// `url` image source block (the Messages API supports URL image sources).
#[test]
fn test_anthropic_user_message_url_image_serializes_url_source() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_binary_url("image/png", "https://example.com/duck.png", None),
	])]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	let content = web_req.payload["messages"][0]["content"]
		.as_array()
		.ok_or("user content must be an array")?;
	assert_eq!(content[0], json!({"type": "text", "text": "What is in this picture?"}));
	assert_eq!(
		content[1],
		json!({
			"type": "image",
			"source": {
				"type": "url",
				"url": "https://example.com/duck.png",
			}
		})
	);

	Ok(())
}

/// Regression guard: a user message with a base64 image must keep the exact
/// base64 `image` source shape (unchanged by the URL-source support).
#[test]
fn test_anthropic_user_message_base64_image_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::user(vec![
		ContentPart::from_text("What is in this picture?"),
		ContentPart::from_binary_base64("image/png", "iVBORw0KBASE64", None),
	])]);
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-haiku-4-5"),
	};

	// -- Exec
	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, chat_req, ChatOptionsSet::default())?;

	// -- Check
	assert_eq!(
		web_req.payload["messages"][0]["content"][1],
		json!({
			"type": "image",
			"source": {
				"type": "base64",
				"media_type": "image/png",
				"data": "iVBORw0KBASE64",
			}
		})
	);

	Ok(())
}

// region:    --- Support

fn build_characterization_payload(model_name: &str, reasoning_effort: Option<ReasoningEffort>) -> Result<Value> {
	let chat_options = reasoning_effort.map(|effort| ChatOptions::default().with_reasoning_effort(effort));
	let options_set = ChatOptionsSet::default().with_chat_options(chat_options.as_ref());
	let target = ServiceTarget {
		endpoint: AnthropicAdapter::default_endpoint(AdapterKind::Anthropic),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::Anthropic, model_name),
	};

	let web_req =
		AnthropicAdapter::to_web_request_data(target, ServiceType::Chat, ChatRequest::from_user("hello"), options_set)?;

	Ok(web_req.payload)
}

// endregion: --- Support
