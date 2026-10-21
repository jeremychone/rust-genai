use super::*;
use crate::chat::{ChatMessage, ChatOptions, JsonSpec, WebSearchConfig};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// region:    --- usageMetadata normalization

/// `toolUsePromptTokenCount` sits OUTSIDE `promptTokenCount`: the page content
/// `urlContext` retrieves is real, billable input that Gemini reports on its own.
/// The numbers are verbatim from a live v1beta `gemini-3.7-flash` call, because the
/// arithmetic between them is the claim — 68 + 50 + 112 + 5858 = 6088, the reported
/// total, which only closes once the tool-use tokens are counted.
#[test]
fn test_gemini_tool_use_prompt_tokens_are_folded_into_prompt_tokens() {
	let usage = GeminiAdapter::into_usage(json!({
		"promptTokenCount": 68,
		"candidatesTokenCount": 50,
		"thoughtsTokenCount": 112,
		"toolUsePromptTokenCount": 5858,
		"totalTokenCount": 6088
	}));

	assert_eq!(usage.prompt_tokens, Some(68 + 5858));
	assert_eq!(usage.completion_tokens, Some(50 + 112));
	assert_eq!(usage.total_tokens, Some(6088));

	// The invariant this fix restores: the normalized halves reconcile with the
	// provider's own total. Without the fold, prompt + completion is 5858 short.
	assert_eq!(
		usage.prompt_tokens.unwrap() + usage.completion_tokens.unwrap(),
		usage.total_tokens.unwrap()
	);
}

/// A call that used no server-side tool must be untouched — `prompt_tokens` left
/// exactly as reported and no details object conjured for it.
#[test]
fn test_gemini_usage_without_tool_use_is_unchanged() {
	let usage = GeminiAdapter::into_usage(json!({
		"promptTokenCount": 10,
		"candidatesTokenCount": 4,
		"totalTokenCount": 14
	}));

	assert_eq!(usage.prompt_tokens, Some(10));
	assert_eq!(usage.completion_tokens, Some(4));
	assert_eq!(usage.prompt_tokens_details, None);
}

/// Cached and tool-use tokens are counted differently and must not be conflated:
/// `cachedContentTokenCount` is already INSIDE `promptTokenCount` (so it is only
/// reported in the details), while `toolUsePromptTokenCount` is outside it (so it
/// is added). Adding both, or neither, is the easy mistake here.
#[test]
fn test_gemini_cached_tokens_are_not_added_but_tool_use_tokens_are() {
	let usage = GeminiAdapter::into_usage(json!({
		"promptTokenCount": 100,
		"cachedContentTokenCount": 80,
		"candidatesTokenCount": 10,
		"toolUsePromptTokenCount": 500,
		"totalTokenCount": 610
	}));

	assert_eq!(usage.prompt_tokens, Some(600), "only tool-use is added, not cached");
	assert_eq!(
		usage.prompt_tokens_details.and_then(|d| d.cached_tokens),
		Some(80),
		"cached tokens still reported as a subset"
	);
}

/// Tool-use tokens with no `promptTokenCount` at all still have to count — dropping
/// them would report a call with thousands of billable input tokens as having none.
#[test]
fn test_gemini_tool_use_tokens_alone_become_the_prompt_count() {
	let usage = GeminiAdapter::into_usage(json!({
		"candidatesTokenCount": 10,
		"toolUsePromptTokenCount": 500,
		"totalTokenCount": 510
	}));
	assert_eq!(usage.prompt_tokens, Some(500));
}

// endregion: --- usageMetadata normalization

#[test]
fn test_gemini_merge_consecutive_tool_responses() {
	let contents = vec![
		json!({"role": "model", "parts": [{"functionCall": {"name": "read", "args": {}}}]}),
		json!({"role": "user", "parts": [{"functionResponse": {"name": "call1", "response": {"content": "a"}}}]}),
		json!({"role": "user", "parts": [{"functionResponse": {"name": "call2", "response": {"content": "b"}}}]}),
	];
	let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
	assert_eq!(merged.len(), 2); // model + single merged user
	let parts = merged[1].get("parts").unwrap().as_array().unwrap();
	assert_eq!(parts.len(), 2); // both functionResponse in one entry
}

#[test]
fn test_gemini_merge_does_not_merge_non_tool_user_entries() {
	let contents = vec![
		json!({"role": "user", "parts": [{"text": "hello"}]}),
		json!({"role": "user", "parts": [{"functionResponse": {"name": "c1", "response": {"content": "x"}}}]}),
	];
	let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
	assert_eq!(merged.len(), 2); // not merged — first is text, not tool response
}

#[test]
fn test_gemini_merge_three_consecutive_tool_responses() {
	let contents = vec![
		json!({"role": "user", "parts": [{"functionResponse": {"name": "c1", "response": {"content": "a"}}}]}),
		json!({"role": "user", "parts": [{"functionResponse": {"name": "c2", "response": {"content": "b"}}}]}),
		json!({"role": "user", "parts": [{"functionResponse": {"name": "c3", "response": {"content": "c"}}}]}),
	];
	let merged = GeminiAdapter::merge_consecutive_tool_response_entries(contents);
	assert_eq!(merged.len(), 1);
	let parts = merged[0].get("parts").unwrap().as_array().unwrap();
	assert_eq!(parts.len(), 3);
}

#[test]
fn test_gemini_function_response_uses_function_name_when_available() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let chat_req = ChatRequest::new(vec![ChatMessage::from(
		ToolResponse::new("call_123", "{}").with_fn_name("get_weather"),
	)]);

	let parts = GeminiAdapter::into_gemini_request_parts(&model_iden, chat_req).unwrap();
	let function_response = &parts.contents[0]["parts"][0]["functionResponse"];

	assert_eq!(function_response["name"], "get_weather");
	assert_eq!(function_response["response"]["name"], "get_weather");
}

#[test]
fn test_gemini_function_response_infers_name_from_gemini_synthetic_call_id() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call#get_weather#0", "{}"))]);

	let parts = GeminiAdapter::into_gemini_request_parts(&model_iden, chat_req).unwrap();
	let function_response = &parts.contents[0]["parts"][0]["functionResponse"];

	assert_eq!(function_response["name"], "get_weather");
	assert_eq!(function_response["response"]["name"], "get_weather");
}

fn build_payload(chat_req: ChatRequest, options: &ChatOptions) -> Value {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-3.7-flash");
	let options_set = ChatOptionsSet::default().with_chat_options(Some(options));
	GeminiAdapter::build_gemini_request_payload(&model_iden, "gemini-3.7-flash", chat_req, options_set)
		.unwrap()
		.0
}

fn web_search_tool() -> Tool {
	Tool::new_web_search().with_config(WebSearchConfig::default())
}

#[test]
fn test_gemini_mixing_builtin_and_function_tools_opts_into_server_side_invocations() {
	let chat_req = ChatRequest::from_user("What is the weather in Cairo?")
		.with_tools(vec![web_search_tool(), Tool::new("get_weather")]);

	let payload = build_payload(chat_req, &ChatOptions::default());

	assert_eq!(payload["toolConfig"]["includeServerSideToolInvocations"], true);
}

#[test]
fn test_gemini_every_builtin_triggers_the_opt_in_when_mixed() {
	for builtin in GEMINI_BUILTIN_TOOL_NAMES {
		let chat_req = ChatRequest::from_user("Do the thing.").with_tools(vec![
			Tool::new(*builtin).with_config(json!({})),
			Tool::new("get_weather"),
		]);
		let payload = build_payload(chat_req, &ChatOptions::default());
		assert_eq!(
			payload["toolConfig"]["includeServerSideToolInvocations"], true,
			"builtin {builtin} should opt in when mixed with function declarations"
		);
	}
}

#[test]
fn test_gemini_unmixed_requests_do_not_get_the_opt_in() {
	// Builtin only.
	let builtin_only = build_payload(
		ChatRequest::from_user("Search for something.").with_tools(vec![web_search_tool()]),
		&ChatOptions::default(),
	);
	assert!(
		builtin_only.get("toolConfig").is_none(),
		"a builtin-only request needs no toolConfig"
	);

	// Function declarations only.
	let functions_only = build_payload(
		ChatRequest::from_user("What is the weather?").with_tools(vec![Tool::new("get_weather")]),
		&ChatOptions::default(),
	);
	assert!(
		functions_only.get("toolConfig").is_none(),
		"a function-only request needs no toolConfig"
	);

	// No tools at all.
	let no_tools = build_payload(ChatRequest::from_user("Hello."), &ChatOptions::default());
	assert!(no_tools.get("toolConfig").is_none());
}

#[test]
fn test_gemini_the_opt_in_merges_with_tool_choice_rather_than_replacing_it() {
	let options = ChatOptions::default().with_tool_choice(ToolChoice::tool("get_weather"));
	let chat_req = ChatRequest::from_user("What is the weather in Cairo?")
		.with_tools(vec![web_search_tool(), Tool::new("get_weather")]);

	let payload = build_payload(chat_req, &options);

	assert_eq!(payload["toolConfig"]["includeServerSideToolInvocations"], true);
	assert_eq!(payload["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
	assert_eq!(
		payload["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"],
		json!(["get_weather"])
	);
}

#[test]
fn test_gemini_tool_choice_required_maps_to_gemini_any_mode() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let options = ChatOptions::default().with_tool_choice(ToolChoice::Required);
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
	let chat_req = ChatRequest::from_user("Use the weather tool.").with_tools(vec![Tool::new("get_weather")]);

	let (payload, _) =
		GeminiAdapter::build_gemini_request_payload(&model_iden, "gemini-2.5-flash", chat_req, options_set).unwrap();

	assert_eq!(payload["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
	assert!(
		payload["toolConfig"]["functionCallingConfig"]
			.get("allowedFunctionNames")
			.is_none()
	);
}

#[test]
fn test_gemini_tool_choice_specific_tool_sets_gemini_allowed_function() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let options = ChatOptions::default().with_tool_choice(ToolChoice::tool("get_weather"));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
	let chat_req = ChatRequest::from_user("Use the weather tool.").with_tools(vec![Tool::new("get_weather")]);

	let (payload, _) =
		GeminiAdapter::build_gemini_request_payload(&model_iden, "gemini-2.5-flash", chat_req, options_set).unwrap();

	assert_eq!(payload["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
	assert_eq!(
		payload["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"],
		json!(["get_weather"])
	);
}

#[test]
fn test_gemini_tool_call_id_uses_counter() {
	// Simulate Gemini response with two functionCalls for the same tool
	let body = json!({
		"candidates": [{
			"content": {
				"role": "model",
				"parts": [
					{"functionCall": {"name": "read_file", "args": {"path": "a.rs"}}},
					{"functionCall": {"name": "read_file", "args": {"path": "b.rs"}}}
				]
			}
		}],
		"usageMetadata": {"totalTokenCount": 100}
	});
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-test");
	let response = GeminiAdapter::body_to_gemini_chat_response(&model_iden, body).unwrap();
	let tool_calls: Vec<_> = response
		.content
		.into_iter()
		.filter_map(|part| match part.data {
			GeminiPartData::ToolCall(tc) => Some(tc),
			_ => None,
		})
		.collect();
	assert_eq!(tool_calls.len(), 2);
	assert_ne!(tool_calls[0].call_id, tool_calls[1].call_id);
	assert!(tool_calls[0].call_id.contains("read_file"));
	assert!(tool_calls[1].call_id.contains("read_file"));
}

#[test]
fn test_gemini_body_to_gemini_chat_response_accepts_usage_only_stream_tail() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let response = GeminiAdapter::body_to_gemini_chat_response(
		&model_iden,
		json!({
			"candidates": [
				{
					"finishReason": "STOP"
				}
			],
			"usageMetadata": {
				"promptTokenCount": 10,
				"candidatesTokenCount": 4,
				"totalTokenCount": 14
			}
		}),
	)
	.expect("usage-only stream tail should not be treated as an error");

	assert!(response.content.is_empty());
	assert_eq!(response.usage.total_tokens, Some(14));
	assert_eq!(response.usage.prompt_tokens, Some(10));
	assert_eq!(response.usage.completion_tokens, Some(4));
	assert_eq!(response.stop_reason.as_deref(), Some("STOP"));
}

#[test]
fn test_gemini_body_to_gemini_chat_response_accepts_tail_with_null_finish_reason() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let response = GeminiAdapter::body_to_gemini_chat_response(
		&model_iden,
		json!({
			"candidates": [
				{
					"finishReason": null
				}
			],
			"usageMetadata": {
				"promptTokenCount": 10,
				"candidatesTokenCount": 4,
				"totalTokenCount": 14
			}
		}),
	)
	.expect("usage-only stream tail with null finishReason should not be an error");

	assert!(response.content.is_empty());
	assert_eq!(response.usage.total_tokens, Some(14));
}

#[test]
fn test_gemini_body_to_gemini_chat_response_still_rejects_missing_candidates() {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let result = GeminiAdapter::body_to_gemini_chat_response(
		&model_iden,
		json!({
			"usageMetadata": {
				"promptTokenCount": 10,
				"candidatesTokenCount": 4,
				"totalTokenCount": 14
			}
		}),
	);

	let err = match result {
		Err(e) => e,
		Ok(_) => panic!("missing candidates should still be rejected"),
	};
	assert!(matches!(err, Error::ChatResponse { .. }));
}

#[test]
fn test_gemini_response_json_schema_is_forwarded_raw() {
	// A pydantic/schemars-style schema with `$defs` + `$ref`. Gemini's
	// `responseJsonSchema` field accepts full JSON Schema, so the adapter must
	// forward it unchanged — no `$ref` inlining / OpenAPI conversion.
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let schema = json!({
		"type": "object",
		"properties": { "person": { "$ref": "#/$defs/Person" } },
		"required": ["person"],
		"$defs": {
			"Person": { "type": "object", "properties": { "name": { "type": "string" } }, "required": ["name"] }
		}
	});
	let options = ChatOptions::default().with_response_format(JsonSpec::new("person", schema.clone()));
	let options_set = ChatOptionsSet::default().with_chat_options(Some(&options));
	let chat_req = ChatRequest::from_user("Return a person.");

	let (payload, _) =
		GeminiAdapter::build_gemini_request_payload(&model_iden, "gemini-2.5-flash", chat_req, options_set).unwrap();

	assert_eq!(payload["generationConfig"]["responseMimeType"], "application/json");
	// Forwarded byte-for-byte: `$defs`/`$ref` preserved, not inlined.
	assert_eq!(payload["generationConfig"]["responseJsonSchema"], schema);
}

#[test]
fn test_gemini_tool_parameters_use_parameters_json_schema_raw() {
	// Tool schemas go to `parametersJsonSchema` (JSON Schema-native), not the
	// restricted `parameters` field (which rejects `$ref`). Forwarded unchanged.
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");
	let schema = json!({
		"type": "object",
		"properties": { "loc": { "$ref": "#/$defs/Loc" } },
		"required": ["loc"],
		"$defs": {
			"Loc": { "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"] }
		}
	});
	let chat_req =
		ChatRequest::from_user("Use the tool.").with_tools(vec![Tool::new("record").with_schema(schema.clone())]);
	let options_set = ChatOptionsSet::default();

	let (payload, _) =
		GeminiAdapter::build_gemini_request_payload(&model_iden, "gemini-2.5-flash", chat_req, options_set).unwrap();

	let decl = &payload["tools"][0]["functionDeclarations"][0];
	assert_eq!(decl["parametersJsonSchema"], schema);
	assert!(
		decl.get("parameters").is_none(),
		"must not use the restricted `parameters` field"
	);
}

// region:    --- thought signatures stay with their part

fn gemini_3() -> ModelIden {
	ModelIden::new(AdapterKind::Gemini, "gemini-3-pro")
}

fn response_with_parts(parts: Value) -> WebResponse {
	WebResponse {
		status: reqwest::StatusCode::OK,
		body: json!({
			"candidates": [{"content": {"parts": parts, "role": "model"}, "finishReason": "STOP"}],
			"usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5, "thoughtsTokenCount": 3},
			"modelVersion": "gemini-3-pro"
		}),
	}
}

fn call(path: &str) -> ToolCall {
	ToolCall {
		call_id: format!("call#read_file#{path}"),
		fn_name: "read_file".to_string(),
		fn_arguments: json!({"path": path}),
		thought_signatures: None,
	}
}

/// A compact rendering of parts: text, signature, or call with its mirrored signatures.
fn shape(parts: &[ContentPart]) -> Vec<String> {
	parts
		.iter()
		.map(|part| match part {
			ContentPart::Text(text) => format!("text({text})"),
			ContentPart::ThoughtSignature(signature) => format!("sig({signature})"),
			ContentPart::ToolCall(call) => {
				let mirrored = call.thought_signatures.as_ref().map(|s| s.join("+")).unwrap_or_default();
				format!("call({}:{mirrored})", call.fn_arguments["path"].as_str().unwrap_or(""))
			}
			other => format!("{other:?}"),
		})
		.collect()
}

/// Gemini attaches a `thoughtSignature` to a specific part and wants it back on
/// that part. Parts therefore keep wire order, with each signature immediately
/// before the part it arrived on and each call mirroring its own — rather than
/// every signature hoisted to the front and all of them on the first call,
/// which lost the association for a turn with two calls.
#[test]
fn test_gemini_signatures_stay_next_to_the_part_they_arrived_on() {
	let response = GeminiAdapter::to_chat_response(
		gemini_3(),
		response_with_parts(json!([
			{"text": "Reading both."},
			{"functionCall": {"name": "read_file", "args": {"path": "/a"}}, "thoughtSignature": "sig-a"},
			{"functionCall": {"name": "read_file", "args": {"path": "/b"}}, "thoughtSignature": "sig-b"}
		])),
		ChatOptionsSet::default(),
	)
	.expect("chat response");

	assert_eq!(
		shape(response.content.parts()),
		[
			"text(Reading both.)",
			"sig(sig-a)",
			"call(/a:sig-a)",
			"sig(sig-b)",
			"call(/b:sig-b)"
		]
	);
}

/// The way back: each call carries the signature that preceded it, embedded in
/// its own `functionCall` part, so neither needs the validator stand-in.
#[test]
fn test_gemini_each_call_goes_back_with_its_own_signature() {
	let content = MessageContent::from_parts(vec![
		ContentPart::Text("Reading both.".to_string()),
		ContentPart::ThoughtSignature("sig-a".to_string()),
		ContentPart::ToolCall(call("/a")),
		ContentPart::ThoughtSignature("sig-b".to_string()),
		ContentPart::ToolCall(call("/b")),
	]);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Read both."), ChatMessage::assistant(content)]);

	let parts = GeminiAdapter::into_gemini_request_parts(&gemini_3(), chat_req).expect("request parts");

	let model_turn = &parts.contents[1]["parts"];
	assert_eq!(model_turn[0], json!({"text": "Reading both."}));
	assert_eq!(model_turn[1]["thoughtSignature"], "sig-a");
	assert_eq!(model_turn[1]["functionCall"]["args"]["path"], "/a");
	assert_eq!(model_turn[2]["thoughtSignature"], "sig-b");
	assert_eq!(model_turn[2]["functionCall"]["args"]["path"], "/b");
}

/// A call Gemini gave an id goes back with that id, and so does its response, so Gemini can
/// match the two. A call that came without one keeps its local stand-in id, which is not sent.
/// A response built from the call id alone takes its name from the call it answers: Gemini
/// rejects a call's id under another name.
#[test]
fn test_gemini_function_call_ids_go_back_with_the_call_and_its_response() {
	let response = GeminiAdapter::to_chat_response(
		gemini_3(),
		response_with_parts(json!([
			{"functionCall": {"name": "read_file", "args": {"path": "/a"}, "id": "call_a"}, "thoughtSignature": "sig-a"},
			{"functionCall": {"name": "list_dir", "args": {"path": "/b"}}}
		])),
		ChatOptionsSet::default(),
	)
	.expect("chat response");
	let calls = response.content.tool_calls();
	assert_eq!(calls[0].call_id, "call_a");
	let tool_responses: Vec<ChatMessage> = calls
		.iter()
		.map(|call| ChatMessage::from(ToolResponse::new(call.call_id.clone(), "{}")))
		.collect();
	let mut messages = vec![ChatMessage::user("Read both."), ChatMessage::assistant(response.content)];
	messages.extend(tool_responses);

	let parts =
		GeminiAdapter::into_gemini_request_parts(&gemini_3(), ChatRequest::new(messages)).expect("request parts");

	let model_turn = &parts.contents[1]["parts"];
	assert_eq!(model_turn[0]["functionCall"]["id"], "call_a");
	assert!(model_turn[1]["functionCall"].get("id").is_none());
	let response_turn = &parts.contents[2]["parts"];
	assert_eq!(response_turn[0]["functionResponse"]["id"], "call_a");
	assert_eq!(response_turn[0]["functionResponse"]["name"], "read_file");
	assert_eq!(response_turn[0]["functionResponse"]["response"]["name"], "read_file");
	assert!(response_turn[1]["functionResponse"].get("id").is_none());
	assert_eq!(response_turn[1]["functionResponse"]["name"], "list_dir");
}

/// A response with no `fn_name` whose call is not in the request has only its call id to go by.
/// That id stands in for the name, as before, and is not sent as the id, since Gemini rejects
/// a call's id under another name.
#[test]
fn test_gemini_a_response_with_no_known_name_is_sent_without_an_id() {
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_123", "{}"))]);

	let parts = GeminiAdapter::into_gemini_request_parts(&gemini_3(), chat_req).expect("request parts");

	let function_response = &parts.contents[0]["parts"][0]["functionResponse"];
	assert_eq!(function_response["name"], "call_123");
	assert!(function_response.get("id").is_none());
}

/// A signature on a text part rides that text part back, as it was returned.
#[test]
fn test_gemini_a_signature_on_a_text_part_rides_the_text_part_back() {
	let content = MessageContent::from_parts(vec![
		ContentPart::ThoughtSignature("sig-t".to_string()),
		ContentPart::Text("Done.".to_string()),
	]);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Hi"), ChatMessage::assistant(content)]);

	let parts = GeminiAdapter::into_gemini_request_parts(&gemini_3(), chat_req).expect("request parts");

	assert_eq!(
		parts.contents[1]["parts"],
		json!([{"text": "Done.", "thoughtSignature": "sig-t"}])
	);
}

/// A call that carries its signature only in the `thought_signatures` mirror —
/// a turn rebuilt from its tool calls alone — still sends it, instead of the
/// Gemini 3 validator stand-in.
#[test]
fn test_gemini_a_mirrored_signature_on_the_call_is_honoured() {
	let mut mirrored = call("/a");
	mirrored.thought_signatures = Some(vec!["sig-m".to_string()]);
	let content = MessageContent::from_parts(vec![ContentPart::ToolCall(mirrored)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::user("Read."), ChatMessage::assistant(content)]);

	let parts = GeminiAdapter::into_gemini_request_parts(&gemini_3(), chat_req).expect("request parts");

	assert_eq!(parts.contents[1]["parts"][0]["thoughtSignature"], "sig-m");
}

/// End to end: a two-call turn goes back in the shape the API returned it.
#[test]
fn test_gemini_a_two_call_turn_round_trips_with_both_signatures() {
	let response = GeminiAdapter::to_chat_response(
		gemini_3(),
		response_with_parts(json!([
			{"functionCall": {"name": "read_file", "args": {"path": "/a"}}, "thoughtSignature": "sig-a"},
			{"functionCall": {"name": "read_file", "args": {"path": "/b"}}, "thoughtSignature": "sig-b"}
		])),
		ChatOptionsSet::default(),
	)
	.expect("chat response");
	let chat_req = ChatRequest::new(vec![
		ChatMessage::user("Read both."),
		ChatMessage::assistant(response.content),
	]);

	let parts = GeminiAdapter::into_gemini_request_parts(&gemini_3(), chat_req).expect("request parts");

	let model_turn = parts.contents[1]["parts"].as_array().expect("parts");
	let signatures: Vec<&str> = model_turn
		.iter()
		.map(|part| part["thoughtSignature"].as_str().unwrap_or("-"))
		.collect();
	assert_eq!(signatures, ["sig-a", "sig-b"]);
	assert!(model_turn.iter().all(|part| part.get("functionCall").is_some()));
}

// endregion: --- thought signatures stay with their part

// region:    --- Thought signature placement

fn signed_call(name: &str, signature: Option<&str>) -> ToolCall {
	ToolCall {
		call_id: format!("call#{name}#0"),
		fn_name: name.to_string(),
		fn_arguments: json!({}),
		thought_signatures: signature.map(|s| vec![s.to_string()]),
	}
}

fn model_parts(parts: Vec<ContentPart>) -> Vec<Value> {
	let chat_req =
		ChatRequest::from_user("hi").append_message(ChatMessage::assistant(MessageContent::from_parts(parts)));
	let payload = build_payload(chat_req, &ChatOptions::default());
	let contents = payload["contents"].as_array().unwrap();
	let model = contents.iter().find(|c| c["role"] == "model").expect("model turn");
	model["parts"].as_array().unwrap().clone()
}

/// Every part carries data; the signature is only ever a field of one.
fn assert_no_bare_signatures(parts: &[Value]) {
	for part in parts {
		assert!(
			part.get("text").is_some() || part.get("functionCall").is_some(),
			"part without data sent: {part}"
		);
	}
}

/// The reported bug: two free signatures before a call. The newer one rides the call and
/// nothing is sent bare.
#[test]
fn test_gemini_free_signatures_before_a_call_are_not_sent_bare() {
	let parts = model_parts(vec![
		ContentPart::ThoughtSignature("sig-1".to_string()),
		ContentPart::ThoughtSignature("sig-2".to_string()),
		ContentPart::ToolCall(signed_call("get_weather", None)),
	]);

	assert_no_bare_signatures(&parts);
	assert_eq!(parts.len(), 1);
	assert_eq!(parts[0]["thoughtSignature"], "sig-2");
}

#[test]
fn test_gemini_streamed_shape_puts_each_signature_on_its_part() {
	let parts = model_parts(vec![
		ContentPart::ThoughtSignature("sig-t".to_string()),
		ContentPart::ThoughtSignature("sig-a".to_string()),
		ContentPart::ThoughtSignature("sig-b".to_string()),
		ContentPart::Text("Reading both.".to_string()),
		ContentPart::ToolCall(signed_call("read_a", Some("sig-a"))),
		ContentPart::ToolCall(signed_call("read_b", Some("sig-b"))),
	]);

	assert_no_bare_signatures(&parts);
	let signatures: Vec<&str> = parts.iter().map(|p| p["thoughtSignature"].as_str().unwrap_or("-")).collect();
	assert_eq!(signatures, ["sig-t", "sig-a", "sig-b"]);
}

#[test]
fn test_gemini_trailing_signature_is_dropped() {
	let parts = model_parts(vec![
		ContentPart::Text("Hmm.".to_string()),
		ContentPart::ThoughtSignature("sig-1".to_string()),
	]);

	assert_eq!(parts, [json!({"text": "Hmm."})]);
}

#[test]
fn test_gemini_unsigned_first_call_gets_the_validator_placeholder() {
	let parts = model_parts(vec![
		ContentPart::ToolCall(signed_call("a", None)),
		ContentPart::ToolCall(signed_call("b", None)),
	]);

	assert_eq!(parts[0]["thoughtSignature"], SKIP_THOUGHT_SIGNATURE_VALIDATOR);
	assert!(parts[1].get("thoughtSignature").is_none());
}

#[test]
fn test_gemini_parser_keeps_a_signature_only_part_as_empty_text() {
	let body = json!({"candidates": [{"content": {"parts": [
		{"thoughtSignature": "sig-t"},
		{"functionCall": {"name": "read_file", "args": {}}, "thoughtSignature": "sig-a"}
	]}}]});
	let response = GeminiAdapter::body_to_gemini_chat_response(&gemini_3(), body).unwrap();

	let [first, second] = response.content.as_slice() else {
		panic!("expected two parts");
	};
	assert!(matches!(&first.data, GeminiPartData::Text(text) if text.is_empty()));
	assert_eq!(first.thought_signature.as_deref(), Some("sig-t"));
	let GeminiPartData::ToolCall(call) = &second.data else {
		panic!("expected a call");
	};
	assert_eq!(call.thought_signatures.as_deref(), Some(&["sig-a".to_string()][..]));
}

// endregion: --- Thought signature placement
#[test]
fn test_gemini_function_response_part_id_only_for_known_calls() -> Result<()> {
	// -- Setup & Fixtures
	let cases = [
		("call_123", Some("get_weather"), false, "get_weather", None),
		("call_123", None, false, "call_123", None),
		("call#get_weather#0", None, false, "get_weather", None),
		("call_123", None, true, "get_weather", Some("call_123")),
		(
			"call_123",
			Some("explicit_name"),
			true,
			"explicit_name",
			Some("call_123"),
		),
		("call#get_weather#0", None, true, "get_weather", None),
	];

	// -- Exec & Check
	for (call_id, fn_name, known, expected_name, expected_id) in cases {
		let mut call_names = HashMap::new();
		if known {
			call_names.insert(call_id.to_string(), "get_weather".to_string());
		}
		let mut response = ToolResponse::new(call_id, "{}");
		if let Some(fn_name) = fn_name {
			response = response.with_fn_name(fn_name);
		}

		let part = gemini_function_response_part(response, &call_names)?;
		let function_response = &part["functionResponse"];

		assert_eq!(function_response["name"], expected_name);
		assert_eq!(function_response["response"]["name"], expected_name);
		assert_eq!(
			function_response.get("id"),
			expected_id.map(|id| json!(id)).as_ref(),
			"call_id={call_id}, known={known}, fn_name={fn_name:?}"
		);
	}

	Ok(())
}

#[test]
fn test_gemini_build_request_payload_supports_seed_and_extra_body() -> Result<()> {
	let chat_req = ChatRequest::new(vec![ChatMessage::user("hello")]);
	let options = ChatOptions::default().with_seed(42).with_extra_body(json!({
		"cachedContent": "cachedContents/abc123",
		"safetySettings": [
			{
				"category": "HARM_CATEGORY_HATE_SPEECH",
				"threshold": "BLOCK_ONLY_HIGH"
			}
		]
	}));

	let payload = build_payload(chat_req, &options);

	assert_eq!(payload["generationConfig"]["seed"], 42);
	assert_eq!(payload["cachedContent"], "cachedContents/abc123");
	assert_eq!(payload["safetySettings"][0]["category"], "HARM_CATEGORY_HATE_SPEECH");

	Ok(())
}

#[test]
fn test_gemini_to_chat_response_reasoning_content_none_when_empty() -> Result<()> {
	let model_iden = ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash");

	let body_no_reasoning = json!({
		"candidates": [{
			"content": {
				"parts": [{"text": "Hello world"}]
			},
			"finishReason": "STOP"
		}]
	});
	let web_response = WebResponse {
		status: reqwest::StatusCode::OK,
		body: body_no_reasoning,
	};
	let chat_res = GeminiAdapter::to_chat_response(model_iden.clone(), web_response, ChatOptionsSet::default())?;
	assert_eq!(chat_res.first_text(), Some("Hello world"));
	assert_eq!(chat_res.reasoning_content, None);

	let body_with_reasoning = json!({
		"candidates": [{
			"content": {
				"parts": [
					{"thought": true, "text": "Thinking first..."},
					{"text": "Final answer"}
				]
			},
			"finishReason": "STOP"
		}]
	});
	let web_response_with_reasoning = WebResponse {
		status: reqwest::StatusCode::OK,
		body: body_with_reasoning,
	};
	let chat_res_with_reasoning =
		GeminiAdapter::to_chat_response(model_iden, web_response_with_reasoning, ChatOptionsSet::default())?;
	assert_eq!(chat_res_with_reasoning.first_text(), Some("Final answer"));
	assert_eq!(
		chat_res_with_reasoning.reasoning_content.as_deref(),
		Some("Thinking first...")
	);

	Ok(())
}
