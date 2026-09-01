use super::GeminiIxAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatMessage, ChatOptions, ChatOptionsSet, ChatRequest, ChatResponseFormat, ChatRole, ContentPart, JsonSpec,
	MessageContent, ReasoningEffort, Tool, ToolCall, ToolChoice, ToolResponse,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{ModelIden, ServiceTarget};
use serde_json::{Value, json};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

const MODEL: &str = "gemini-3.5-flash";

// region:    --- Request — base payload

#[test]
fn test_gemini_ix_base_payload_and_headers() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Hello")]).with_system("Be brief.");

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["model"], MODEL);
	assert_eq!(request.payload["stream"], false);
	assert_eq!(request.payload["system_instruction"], "Be brief.");
	assert_eq!(request.payload["input"][0]["type"], "user_input");
	assert_eq!(request.payload["input"][0]["content"][0]["type"], "text");
	assert_eq!(request.payload["input"][0]["content"][0]["text"], "Hello");

	// The URL is the same for chat and stream — `stream` is a body field, not an endpoint.
	assert_eq!(
		request.url,
		"https://generativelanguage.googleapis.com/v1beta/interactions"
	);

	let headers: Vec<(&str, &str)> = request.headers.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
	assert!(
		headers.contains(&("x-goog-api-key", "test-key")),
		"headers: {headers:?}"
	);
	assert!(
		headers.contains(&("Api-Revision", GeminiIxAdapter::API_REVISION)),
		"the Api-Revision header pins the steps schema; headers: {headers:?}"
	);

	Ok(())
}

#[test]
fn test_gemini_ix_stream_flag_shares_the_chat_url() -> Result<()> {
	// -- Exec
	let request = support_request(ChatRequest::from_user("Hello"), None, ServiceType::ChatStream)?;

	// -- Check
	assert_eq!(request.payload["stream"], true);
	assert_eq!(
		request.url,
		"https://generativelanguage.googleapis.com/v1beta/interactions"
	);

	Ok(())
}

#[test]
fn test_gemini_ix_inline_system_messages_join_into_system_instruction() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("First."),
		ChatMessage::system("Second."),
		ChatMessage::user("Hello"),
	])
	.with_system("Root.");

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["system_instruction"], "Root.\n\nFirst.\n\nSecond.");
	// System never becomes a step — the Interactions API has no system step type.
	assert_eq!(request.payload["input"].as_array().map(Vec::len), Some(1));

	Ok(())
}

// endregion: --- Request — base payload

// region:    --- Request — stateful session

#[test]
fn test_gemini_ix_previous_interaction_id_and_store() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::from_user("And then?")
		.with_previous_response_id("v1_ChdXS0l4YWZXTk9xbk0")
		.with_store(true);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["previous_interaction_id"], "v1_ChdXS0l4YWZXTk9xbk0");
	assert_eq!(request.payload["store"], true);

	Ok(())
}

#[test]
fn test_gemini_ix_store_defaults_to_true() -> Result<()> {
	// -- Exec
	let request = support_request(ChatRequest::from_user("Hello"), None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["store"], true);
	assert!(request.payload.get("previous_interaction_id").is_none());

	Ok(())
}

#[test]
fn test_gemini_ix_store_can_be_opted_out() -> Result<()> {
	// -- Exec
	let chat_req = ChatRequest::from_user("Hello").with_store(false);
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["store"], false);

	Ok(())
}

// endregion: --- Request — stateful session

// region:    --- Request — steps

#[test]
fn test_gemini_ix_tool_roundtrip_builds_ordered_steps() -> Result<()> {
	// -- Setup & Fixtures
	// A full assistant turn: thought → tool call, then the tool result coming back.
	let assistant = ChatMessage {
		role: ChatRole::Assistant,
		content: MessageContent::from_parts(vec![
			ContentPart::ThoughtSignature("sig-abc".to_string()),
			ContentPart::ToolCall(ToolCall {
				call_id: "call_1".to_string(),
				fn_name: "get_weather".to_string(),
				fn_arguments: json!({"city": "Chennai"}),
				thought_signatures: None,
			}),
		]),
		options: None,
	};
	let tool_result = ChatMessage::from(ToolResponse::new("call_1", r#"{"c":31}"#).with_fn_name("get_weather"));

	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather?"), assistant, tool_result]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let steps = request.payload["input"].as_array().ok_or("input should be an array")?;
	assert_eq!(steps.len(), 4, "steps: {steps:#?}");

	assert_eq!(steps[0]["type"], "user_input");

	// The thought must precede its function_call — the docs require model steps to be echoed
	// back in the order they were received.
	assert_eq!(steps[1]["type"], "thought");
	assert_eq!(steps[1]["signature"], "sig-abc");

	assert_eq!(steps[2]["type"], "function_call");
	assert_eq!(steps[2]["id"], "call_1");
	assert_eq!(steps[2]["name"], "get_weather");
	// `arguments` is a JSON object on the wire, not a string (unlike OpenAI).
	assert_eq!(steps[2]["arguments"]["city"], "Chennai");

	assert_eq!(steps[3]["type"], "function_result");
	assert_eq!(steps[3]["call_id"], "call_1");
	assert_eq!(steps[3]["name"], "get_weather");
	assert_eq!(steps[3]["result"][0]["type"], "text");
	assert_eq!(steps[3]["result"][0]["text"], r#"{"c":31}"#);

	Ok(())
}

/// Verified against the live API: the provider rejects a replayed `function_call` that is not
/// preceded by a `thought` step in its own turn. `into_tool_calls()` drops the standalone
/// `ThoughtSignature` parts, so the signature has to survive on the `ToolCall` itself.
#[test]
fn test_gemini_ix_tool_call_signature_emits_its_thought_step() -> Result<()> {
	// -- Setup & Fixtures
	let assistant = ChatMessage {
		role: ChatRole::Assistant,
		content: MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({"city": "Chennai"}),
			thought_signatures: Some(vec!["sig-from-tool-call".to_string()]),
		})]),
		options: None,
	};
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather?"), assistant]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let steps = request.payload["input"].as_array().ok_or("input should be an array")?;
	assert_eq!(steps.len(), 3, "steps: {steps:#?}");
	assert_eq!(steps[1]["type"], "thought");
	assert_eq!(steps[1]["signature"], "sig-from-tool-call");
	assert_eq!(steps[2]["type"], "function_call");

	Ok(())
}

/// A signature carried both as a standalone part and on the tool call must not be sent twice.
#[test]
fn test_gemini_ix_thought_signature_is_not_emitted_twice() -> Result<()> {
	// -- Setup & Fixtures
	let assistant = ChatMessage {
		role: ChatRole::Assistant,
		content: MessageContent::from_parts(vec![
			ContentPart::ThoughtSignature("sig-abc".to_string()),
			ContentPart::ToolCall(ToolCall {
				call_id: "call_1".to_string(),
				fn_name: "get_weather".to_string(),
				fn_arguments: json!({}),
				// This is exactly the shape `to_chat_response` produces.
				thought_signatures: Some(vec!["sig-abc".to_string()]),
			}),
		]),
		options: None,
	};
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather?"), assistant]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let steps = request.payload["input"].as_array().ok_or("input should be an array")?;
	let thoughts: Vec<&Value> = steps.iter().filter(|step| step["type"] == "thought").collect();
	assert_eq!(thoughts.len(), 1, "steps: {steps:#?}");
	assert_eq!(thoughts[0]["signature"], "sig-abc");

	Ok(())
}

/// A hand-built tool call has no signature at all. The provider accepts a documented sentinel in
/// place of one — without it the request is a 400.
#[test]
fn test_gemini_ix_unsigned_tool_call_uses_the_skip_sentinel() -> Result<()> {
	// -- Setup & Fixtures
	let assistant = ChatMessage {
		role: ChatRole::Assistant,
		content: MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({}),
			thought_signatures: None,
		})]),
		options: None,
	};
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather?"), assistant]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let steps = request.payload["input"].as_array().ok_or("input should be an array")?;
	assert_eq!(steps[1]["type"], "thought");
	assert_eq!(steps[1]["signature"], "skip_thought_signature_validator");
	assert_eq!(steps[2]["type"], "function_call");

	Ok(())
}

#[test]
fn test_gemini_ix_thinking_summaries_opt_in() -> Result<()> {
	// -- Setup & Fixtures
	// Without `thinking_summaries`, `thought` steps carry a signature only and
	// `reasoning_content` comes back empty. Both reasoning options must turn it on.
	let cases = [
		(ChatOptions::default(), None),
		(
			ChatOptions::default().with_capture_reasoning_content(true),
			Some("auto"),
		),
		(
			ChatOptions::default().with_normalize_reasoning_content(true),
			Some("auto"),
		),
	];

	for (options, expected) in cases {
		// -- Exec
		let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

		// -- Check
		let summaries = request.payload["generation_config"]
			.get("thinking_summaries")
			.and_then(Value::as_str);
		assert_eq!(summaries, expected);
	}

	Ok(())
}

/// Verified against the live API: `function_result.name` is required in practice, even though the
/// API reference marks it optional. `ToolResponse::fn_name` is optional in genai, so the name has
/// to be recovered from the `function_call` the result answers.
#[test]
fn test_gemini_ix_function_result_name_resolves_from_the_call() -> Result<()> {
	// -- Setup & Fixtures
	// `ToolResponse::new` carries no fn_name — this is what `common_test_tool_full_flow_ok` builds.
	let assistant = ChatMessage {
		role: ChatRole::Assistant,
		content: MessageContent::from_parts(vec![ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({"city": "Chennai"}),
			thought_signatures: Some(vec!["sig-abc".to_string()]),
		})]),
		options: None,
	};
	let tool_result = ChatMessage::from(ToolResponse::new("call_1", r#"{"c":31}"#));
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Weather?"), assistant, tool_result]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let steps = request.payload["input"].as_array().ok_or("input should be an array")?;
	let result_step = steps.last().ok_or("should have a function_result step")?;
	assert_eq!(result_step["type"], "function_result");
	assert_eq!(result_step["call_id"], "call_1");
	assert_eq!(
		result_step["name"], "get_weather",
		"the name must be recovered from the matching function_call"
	);

	Ok(())
}

/// The stateful tool loop: only the result travels, so the matching `function_call` is not in the
/// request and the name cannot be recovered. The provider rejects both a missing name and a wrong
/// one, so guessing is not an option — fail with something the caller can act on.
#[test]
fn test_gemini_ix_unresolvable_function_result_name_errors() -> Result<()> {
	// -- Setup & Fixtures
	// A stateful continuation: previous_interaction_id + a bare tool result, no assistant turn.
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", r#"{"c":31}"#))])
		.with_previous_response_id("v1_abc")
		.with_store(true);

	// -- Exec
	let res = support_request(chat_req, None, ServiceType::Chat);

	// -- Check
	let Err(err) = res else {
		return Err("should refuse to build a request with an unresolvable tool name".into());
	};
	let message = err.to_string();
	assert!(message.contains("call_1"), "should name the offending call: {message}");
	assert!(message.contains("from_tool_call"), "should point at the fix: {message}");

	Ok(())
}

/// The same shape is fine once the caller supplies the name.
#[test]
fn test_gemini_ix_stateful_function_result_with_fn_name_ok() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", r#"{"c":31}"#).with_fn_name("get_weather");
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)])
		.with_previous_response_id("v1_abc")
		.with_store(true);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let step = &request.payload["input"][0];
	assert_eq!(step["type"], "function_result");
	assert_eq!(step["name"], "get_weather");
	assert_eq!(request.payload["previous_interaction_id"], "v1_abc");

	Ok(())
}

#[test]
fn test_gemini_ix_audio_binary_becomes_an_audio_content_block() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::user(ContentPart::from_binary_base64(
		"audio/mp3",
		"QUJD",
		None,
	))]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let content = &request.payload["input"][0]["content"][0];
	assert_eq!(content["type"], "audio");
	assert_eq!(content["mime_type"], "audio/mp3");
	assert_eq!(content["data"], "QUJD");

	Ok(())
}

#[test]
fn test_gemini_ix_binary_kinds_map_by_mime_type() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("image/png", "image"),
		("audio/wav", "audio"),
		("video/mp4", "video"),
		("application/pdf", "document"),
		("text/csv", "document"),
	];

	for (mime_type, expected_kind) in cases {
		// -- Exec
		let chat_req = ChatRequest::new(vec![ChatMessage::user(ContentPart::from_binary_base64(
			mime_type, "QUJD", None,
		))]);
		let request = support_request(chat_req, None, ServiceType::Chat)?;

		// -- Check
		assert_eq!(
			request.payload["input"][0]["content"][0]["type"], expected_kind,
			"for mime_type '{mime_type}'"
		);
	}

	Ok(())
}

#[test]
fn test_gemini_ix_binary_url_becomes_a_uri_reference() -> Result<()> {
	// -- Setup & Fixtures
	// This is the shape a Files API reference takes (see plans/gemini-interactions-adapter.md §5b).
	let chat_req = ChatRequest::new(vec![ChatMessage::user(ContentPart::from_binary_url(
		"audio/mp3",
		"https://generativelanguage.googleapis.com/v1beta/files/abc123",
		None,
	))]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	let content = &request.payload["input"][0]["content"][0];
	assert_eq!(content["type"], "audio");
	assert_eq!(
		content["uri"],
		"https://generativelanguage.googleapis.com/v1beta/files/abc123"
	);
	assert!(content.get("data").is_none());

	Ok(())
}

// endregion: --- Request — steps

// region:    --- Request — tools & formats

#[test]
fn test_gemini_ix_function_tools_are_flat() -> Result<()> {
	// -- Setup & Fixtures
	let tool = Tool::new("get_weather")
		.with_description("Get the weather")
		.with_schema(json!({"type": "object", "properties": {"city": {"type": "string"}}}));
	let chat_req = ChatRequest::from_user("Weather?").with_tools(vec![tool]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	// No `functionDeclarations` nesting here, unlike the generateContent protocol.
	let tool = &request.payload["tools"][0];
	assert_eq!(tool["type"], "function");
	assert_eq!(tool["name"], "get_weather");
	assert_eq!(tool["description"], "Get the weather");
	assert_eq!(tool["parameters"]["properties"]["city"]["type"], "string");
	assert!(tool.get("functionDeclarations").is_none());

	Ok(())
}

#[test]
fn test_gemini_ix_builtin_tools_are_bare_type_tags() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::from_user("Search it").with_tools(vec![
		Tool::new(crate::chat::ToolName::WebSearch),
		Tool::new("code_execution"),
	]);

	// -- Exec
	let request = support_request(chat_req, None, ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["tools"][0], json!({"type": "google_search"}));
	assert_eq!(request.payload["tools"][1], json!({"type": "code_execution"}));

	Ok(())
}

#[test]
fn test_gemini_ix_tool_choice_variants() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		(ToolChoice::Auto, json!("auto")),
		(ToolChoice::None, json!("none")),
		(ToolChoice::Required, json!("any")),
		(
			ToolChoice::tool("get_weather"),
			json!({"allowed_tools": {"mode": "any", "tools": ["get_weather"]}}),
		),
	];

	for (tool_choice, expected) in cases {
		// -- Exec
		let options = ChatOptions::default().with_tool_choice(tool_choice);
		let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

		// -- Check
		assert_eq!(request.payload["generation_config"]["tool_choice"], expected);
	}

	Ok(())
}

#[test]
fn test_gemini_ix_response_format_rides_the_text_variant() -> Result<()> {
	// -- Setup & Fixtures
	let schema = json!({"type": "object", "properties": {"answer": {"type": "string"}}});
	let json_spec = JsonSpec::new("answer_spec", schema.clone());
	let options = ChatOptions::default().with_response_format(ChatResponseFormat::JsonSpec(json_spec));

	// -- Exec
	let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

	// -- Check
	// ResponseFormat is polymorphic on output *modality*, not a JSON-schema wrapper.
	let response_format = &request.payload["response_format"];
	assert_eq!(response_format["type"], "text");
	assert_eq!(response_format["mime_type"], "application/json");
	assert_eq!(response_format["schema"], schema);

	Ok(())
}

#[test]
fn test_gemini_ix_reasoning_effort_maps_to_thinking_level() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		(ReasoningEffort::Zero, Some("minimal")),
		(ReasoningEffort::Minimal, Some("minimal")),
		(ReasoningEffort::Low, Some("low")),
		(ReasoningEffort::Medium, Some("medium")),
		(ReasoningEffort::High, Some("high")),
		(ReasoningEffort::Max, Some("high")),
		// The Interactions API exposes discrete levels only — a token budget has no equivalent,
		// so it falls back to the middle level rather than dropping the caller's intent.
		(ReasoningEffort::Budget(4096), Some("medium")),
	];

	for (reasoning_effort, expected) in cases {
		// -- Exec
		let options = ChatOptions::default().with_reasoning_effort(reasoning_effort.clone());
		let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

		// -- Check
		let thinking_level = request.payload["generation_config"]
			.get("thinking_level")
			.and_then(Value::as_str);
		assert_eq!(thinking_level, expected, "for {reasoning_effort:?}");
	}

	Ok(())
}

// endregion: --- Request — tools & formats

// region:    --- Request — extra_body deep merge

#[test]
fn test_gemini_ix_extra_body_deep_merges_into_generation_config() -> Result<()> {
	// -- Setup & Fixtures
	// This is how transcription is configured — the headline use of extra_body for this adapter.
	// A shallow merge (`Value::x_merge`, what every other adapter uses) would replace the whole
	// `generation_config` object and silently drop max_output_tokens / thinking_level.
	let options = ChatOptions::default()
		.with_max_tokens(1024)
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_extra_body(json!({
			"generation_config": {
				"transcription_config": {
					"language_codes": ["en-US"],
					"mode": {"type": "verbatim", "diarization_mode": "speaker"},
				}
			}
		}));

	// -- Exec
	let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

	// -- Check
	let generation_config = &request.payload["generation_config"];
	assert_eq!(generation_config["max_output_tokens"], 1024, "must survive the merge");
	assert_eq!(generation_config["thinking_level"], "low", "must survive the merge");
	assert_eq!(generation_config["transcription_config"]["language_codes"][0], "en-US");
	assert_eq!(
		generation_config["transcription_config"]["mode"]["diarization_mode"],
		"speaker"
	);

	Ok(())
}

#[test]
fn test_gemini_ix_extra_body_can_still_override_a_scalar() -> Result<()> {
	// -- Setup & Fixtures
	let options = ChatOptions::default()
		.with_max_tokens(1024)
		.with_extra_body(json!({"generation_config": {"max_output_tokens": 42}, "store": true}));

	// -- Exec
	let request = support_request(ChatRequest::from_user("Hello"), Some(options), ServiceType::Chat)?;

	// -- Check
	assert_eq!(request.payload["generation_config"]["max_output_tokens"], 42);
	assert_eq!(request.payload["store"], true);

	Ok(())
}

// endregion: --- Request — extra_body deep merge

// region:    --- Response

#[test]
fn test_gemini_ix_to_chat_response_walks_the_step_timeline() -> Result<()> {
	// -- Setup & Fixtures
	let body = json!({
		"id": "v1_ChdXS0l4YWZXTk9xbk0",
		"object": "interaction",
		"model": "gemini-3.5-flash",
		"status": "completed",
		"steps": [
			{"type": "thought", "signature": "sig-xyz", "summary": [{"type": "text", "text": "Thinking..."}]},
			{"type": "model_output", "content": [{"type": "text", "text": "The answer is 42."}]}
		],
		"usage": {
			"total_input_tokens": 7,
			"total_output_tokens": 23,
			"total_thought_tokens": 49,
			"total_cached_tokens": 0,
			"total_tool_use_tokens": 0,
			"total_tokens": 79
		}
	});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	assert_eq!(chat_res.first_text(), Some("The answer is 42."));
	assert_eq!(chat_res.reasoning_content.as_deref(), Some("Thinking..."));
	// The signature is what continuity needs in stateless mode.
	assert_eq!(chat_res.content.thought_signatures(), vec!["sig-xyz"]);
	// The interaction id feeds the next turn's `previous_response_id`.
	assert_eq!(chat_res.response_id.as_deref(), Some("v1_ChdXS0l4YWZXTk9xbk0"));
	// NOTE: id is absent when the interaction was not stored — see the next test.
	assert_eq!(chat_res.provider_model_iden.model_name.as_str(), "gemini-3.5-flash");

	// Usage is normalized the "OpenAI way": thought tokens are folded into completion_tokens
	// and broken out in the details.
	assert_eq!(chat_res.usage.prompt_tokens, Some(7));
	assert_eq!(chat_res.usage.completion_tokens, Some(23 + 49));
	assert_eq!(
		chat_res.usage.completion_tokens_details.and_then(|d| d.reasoning_tokens),
		Some(49)
	);
	assert_eq!(chat_res.usage.total_tokens, Some(79));

	Ok(())
}

#[test]
fn test_gemini_ix_to_chat_response_without_id_when_not_stored() -> Result<()> {
	// -- Setup & Fixtures
	// Verified against the live API: an unstored interaction (`store: false`) comes back with no
	// `id` at all, even though the API reference marks the field required. Parsing must not fail.
	let body = json!({
		"object": "interaction",
		"model": "gemini-3.5-flash",
		"status": "completed",
		"steps": [{"type": "model_output", "content": [{"type": "text", "text": "Hi."}]}],
		"usage": {"total_input_tokens": 10, "total_output_tokens": 34, "total_tokens": 525, "raw_prompt_token": 41}
	});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	assert_eq!(chat_res.first_text(), Some("Hi."));
	assert_eq!(
		chat_res.response_id, None,
		"nothing was stored, so there is nothing to continue"
	);
	// An unknown usage field (`raw_prompt_token`) must not break parsing either.
	assert_eq!(chat_res.usage.prompt_tokens, Some(10));

	Ok(())
}

#[test]
fn test_gemini_ix_to_chat_response_extracts_tool_calls() -> Result<()> {
	// -- Setup & Fixtures
	let body = json!({
		"id": "v1_abc",
		"status": "requires_action",
		"steps": [
			{"type": "thought", "signature": "sig-xyz"},
			{"type": "function_call", "id": "call_9", "name": "get_weather", "arguments": {"city": "Chennai"}}
		]
	});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	let tool_calls = chat_res.content.tool_calls();
	assert_eq!(tool_calls.len(), 1);
	assert_eq!(tool_calls[0].call_id, "call_9");
	assert_eq!(tool_calls[0].fn_name, "get_weather");
	assert_eq!(tool_calls[0].fn_arguments["city"], "Chennai");
	// The signature must ride the tool call, since `into_tool_calls()` drops the standalone part
	// and the API rejects a replayed function_call without its thought step.
	assert_eq!(
		tool_calls[0].thought_signatures.as_deref(),
		Some(["sig-xyz".to_string()].as_slice())
	);
	// `requires_action` means the model is waiting on a client-side function_result.
	assert!(
		matches!(chat_res.stop_reason, Some(crate::chat::StopReason::ToolCall(_))),
		"stop_reason: {:?}",
		chat_res.stop_reason
	);

	Ok(())
}

#[test]
fn test_gemini_ix_to_chat_response_skips_unknown_steps() -> Result<()> {
	// -- Setup & Fixtures
	// Built-in tool steps have no genai counterpart and must never fail the response.
	let body = json!({
		"id": "v1_abc",
		"status": "completed",
		"steps": [
			{"type": "google_search_call", "id": "s1", "arguments": {"queries": ["weather"]}},
			{"type": "google_search_result", "id": "s1", "signature": "sig"},
			{"type": "some_step_invented_next_year", "whatever": true},
			{"type": "model_output", "content": [{"type": "text", "text": "Sunny."}]}
		]
	});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	assert_eq!(chat_res.first_text(), Some("Sunny."));

	Ok(())
}

#[test]
fn test_gemini_ix_to_chat_response_maps_media_output() -> Result<()> {
	// -- Setup & Fixtures
	let body = json!({
		"id": "v1_abc",
		"status": "completed",
		"steps": [
			{"type": "model_output", "content": [
				{"type": "image", "mime_type": "image/png", "data": "QUJD"}
			]}
		]
	});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	let binaries = chat_res.content.binaries();
	assert_eq!(binaries.len(), 1);
	assert_eq!(binaries[0].content_type, "image/png");

	Ok(())
}

#[test]
fn test_gemini_ix_to_chat_response_incomplete_is_max_tokens() -> Result<()> {
	// -- Setup & Fixtures
	let body = json!({"id": "v1_abc", "status": "incomplete", "steps": []});

	// -- Exec
	let chat_res = support_response(body)?;

	// -- Check
	assert!(
		matches!(chat_res.stop_reason, Some(crate::chat::StopReason::MaxTokens(_))),
		"stop_reason: {:?}",
		chat_res.stop_reason
	);

	Ok(())
}

// endregion: --- Response

// region:    --- Routing

#[test]
fn test_gemini_ix_model_routing() -> Result<()> {
	// -- Setup & Fixtures
	// The Interactions API is opt-in only. A bare model name never selects it, so adding this
	// adapter cannot change the protocol under an existing caller.
	let cases = [
		// -- Namespaced: opts in
		("gemini_ix::gemini-3.5-flash", AdapterKind::GeminiIx),
		("gemini_ix::gemini-3.5-transcribe", AdapterKind::GeminiIx),
		("gemini_ix::gemini-2.5-flash", AdapterKind::GeminiIx),
		("gemini_interactions::gemini-3.5-flash", AdapterKind::Gemini),
		// -- Bare names always stay on generateContent, Gemini 3.x included
		("gemini-3.5-flash", AdapterKind::Gemini),
		("gemini-3.5-transcribe", AdapterKind::Gemini),
		("gemini-3.1-pro-preview", AdapterKind::Gemini),
		("gemini-2.5-flash", AdapterKind::Gemini),
		("gemini-flash-latest", AdapterKind::Gemini),
		("gemini-embedding-001", AdapterKind::Gemini),
		("gemini::gemini-3.5-flash", AdapterKind::Gemini),
	];

	for (model, expected) in cases {
		// -- Exec & Check
		assert_eq!(AdapterKind::from_model(model)?, expected, "for model '{model}'");
	}

	Ok(())
}

#[test]
fn test_gemini_ix_embeddings_are_not_supported() -> Result<()> {
	// -- Exec
	let res = GeminiIxAdapter::get_service_url(
		&ModelIden::new(AdapterKind::GeminiIx, MODEL),
		ServiceType::Embed,
		Endpoint::from_static("https://generativelanguage.googleapis.com/v1beta/"),
	);

	// -- Check
	assert!(
		matches!(res, Err(crate::Error::AdapterNotSupported { .. })),
		"embeddings have no Interactions endpoint"
	);

	Ok(())
}

// endregion: --- Routing

// region:    --- Support

fn support_request(
	chat_req: ChatRequest,
	chat_options: Option<ChatOptions>,
	service_type: ServiceType,
) -> Result<WebRequestData> {
	let options_set = ChatOptionsSet::default().with_chat_options(chat_options.as_ref());

	let request = GeminiIxAdapter::to_web_request_data(
		ServiceTarget {
			model: ModelIden::new(AdapterKind::GeminiIx, MODEL),
			auth: AuthData::from_single("test-key"),
			endpoint: Endpoint::from_static("https://generativelanguage.googleapis.com/v1beta/"),
		},
		service_type,
		chat_req,
		options_set,
	)?;

	Ok(request)
}

fn support_response(body: Value) -> Result<crate::chat::ChatResponse> {
	let chat_res = GeminiIxAdapter::to_chat_response(
		ModelIden::new(AdapterKind::GeminiIx, MODEL),
		WebResponse {
			status: reqwest::StatusCode::OK,
			body,
		},
		ChatOptionsSet::default(),
	)?;

	Ok(chat_res)
}

// endregion: --- Support
