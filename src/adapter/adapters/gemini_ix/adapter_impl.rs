use super::GeminiIxStreamer;
use super::ix_types::{IxInteraction, ix_status_to_stop_reason};
use crate::adapter::adapters::gemini::GeminiAdapter;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatMessage, ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole,
	ChatStream, ChatStreamResponse, ContentPart, MessageContent, ReasoningEffort, Tool, ToolChoice, ToolName,
	ToolResponse, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{EventSourceStream, WebClient, WebResponse};
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use value_ext::JsonValueExt;

pub struct GeminiIxAdapter;

impl GeminiIxAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GEMINI_API_KEY";

	pub const API_REVISION: &str = "2026-05-20";
}

/// Server-side tools, passed as a bare `{"type": "<name>"}` tag rather than a function
/// declaration. The `generateContent` adapter carries the same list in camelCase
/// (`GEMINI_BUILTIN_TOOL_NAMES`); the Interactions API uses snake_case.
/// Stand-in accepted by the provider when a `function_call` has to be replayed without a real
/// thought signature. Similar to how generateContent works
/// DOC: <https://ai.google.dev/gemini-api/docs/thought-signatures#faqs>
const SKIP_THOUGHT_SIGNATURE_VALIDATOR: &str = "skip_thought_signature_validator";

const IX_BUILTIN_TOOL_NAMES: &[&str] = &[
	"google_search",
	"code_execution",
	"url_context",
	"file_search",
	"computer_use",
	"google_maps",
	"mcp_server",
	"retrieval",
];

/// `generation_config.tool_choice` accepts either a bare enum string or a `ToolChoiceConfig`.
/// DOC: <https://ai.google.dev/api/interactions#Resource:ToolChoiceConfig>
fn ix_tool_choice(tool_choice: Option<&ToolChoice>) -> Option<Value> {
	match tool_choice? {
		ToolChoice::Auto => Some(json!("auto")),
		ToolChoice::None => Some(json!("none")),
		ToolChoice::Required => Some(json!("any")),
		ToolChoice::Tool { name } => Some(json!({
			"allowed_tools": {
				"mode": "any",
				"tools": [name],
			}
		})),
	}
}

fn deep_merge(target: &mut Value, overlay: Value) {
	match (target, overlay) {
		(Value::Object(target_map), Value::Object(overlay_map)) => {
			for (key, overlay_value) in overlay_map {
				match target_map.get_mut(&key) {
					Some(target_value) => deep_merge(target_value, overlay_value),
					None => {
						target_map.insert(key, overlay_value);
					}
				}
			}
		}
		(target, overlay) => *target = overlay,
	}
}

/// Maps the normalized reasoning effort onto `generation_config.thinking_level`.
///
/// The Interactions API exposes discrete levels only. there is no token-budget equivalent, so
/// `ReasoningEffort::Budget` has nothing to map onto.
fn ix_thinking_level(reasoning_effort: Option<&ReasoningEffort>) -> Option<&'static str> {
	match reasoning_effort? {
		ReasoningEffort::Zero | ReasoningEffort::Minimal => Some("minimal"),
		ReasoningEffort::Low => Some("low"),
		ReasoningEffort::Medium => Some("medium"),
		ReasoningEffort::High | ReasoningEffort::XHigh | ReasoningEffort::Max => Some("high"),
		ReasoningEffort::Budget(budget) => {
			tracing::warn!(
				"GeminiIx - ReasoningEffort::Budget({budget}) is not supported by the Interactions API \
				 (it exposes discrete `thinking_level` values only)"
			);
			Some("medium")
		}
	}
}

impl Adapter for GeminiIxAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_auth(_kind: AdapterKind) -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
		Endpoint::from_static(BASE_URL)
	}

	async fn all_model_names(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		GeminiAdapter::all_model_names(kind, endpoint, auth, web_client).await
	}

	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => {
				let base_url = endpoint.base_url();
				let base_url = reqwest::Url::parse(base_url)
					.map_err(|err| Error::Internal(format!("Cannot parse url: {base_url}. Cause:\n{err}")))?;
				let original_query_params = base_url.query().to_owned();

				let mut full_url = base_url.join("interactions").map_err(|err| {
					Error::Internal(format!(
						"Cannot join url suffix 'interactions' for base_url '{base_url}'. Cause:\n{err}"
					))
				})?;
				full_url.set_query(original_query_params);
				Ok(full_url.to_string())
			}
			ServiceType::Embed => Err(Error::AdapterNotSupported {
				adapter_kind: AdapterKind::GeminiIx,
				feature: "embeddings".to_string(),
			}),
		}
	}

	/// Google Doc: <https://ai.google.dev/gemini-api/docs/interactions>
	/// API Reference: <https://ai.google.dev/api/interactions>
	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;
		let (_, model_name) = model.model_name.namespace_and_name();

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint)?;

		// -- headers
		let headers = Headers::from([("x-goog-api-key", api_key), ("Api-Revision", Self::API_REVISION.to_string())]);

		let stream = matches!(service_type, ServiceType::ChatStream);

		let ChatRequest {
			system,
			messages,
			tools,
			previous_response_id,
			store: explicit_store,
		} = chat_req;

		// -- Build the input steps (and collect any inline system messages)
		let mut systems: Vec<String> = Vec::new();
		if let Some(system) = system {
			systems.push(system);
		}
		let input_steps = Self::into_ix_input_steps(&model, messages, &mut systems)?;

		let store = explicit_store.unwrap_or(true);
		if previous_response_id.is_some() && explicit_store == Some(false) {
			tracing::warn!(
				"previous_response_id is set together with store=false. Gemini Interactions API \
				 cannot resolve a previous interaction that was not stored, and store=false also \
				 prevents this interaction from being continued."
			);
		}

		let mut payload = json!({
			"model": model_name,
			"store": store,
			"stream": stream,
		});

		// -- System prompt (interaction-scoped: re-sent every turn)
		if !systems.is_empty() {
			payload.x_insert("system_instruction", systems.join("\n\n"))?;
		}

		// -- Stateful session
		if let Some(previous_interaction_id) = previous_response_id.as_deref() {
			payload.x_insert("previous_interaction_id", previous_interaction_id)?;
		}

		// -- Tools (interaction-scoped)
		if let Some(tools) = tools {
			payload.x_insert("tools", Self::into_ix_tools(tools))?;
		}

		// -- Input steps
		payload.x_insert("input", input_steps)?;

		// -- Response format
		if let Some(response_format) = Self::into_ix_response_format(chat_options.response_format()) {
			payload.x_insert("response_format", response_format)?;
		}

		// -- generation_config (interaction-scoped)
		let mut generation_config = Map::new();

		if let Some(max_tokens) = chat_options.max_tokens() {
			generation_config.insert("max_output_tokens".into(), max_tokens.into());
		}
		if !chat_options.stop_sequences().is_empty() {
			generation_config.insert("stop_sequences".into(), chat_options.stop_sequences().into());
		}
		if let Some(seed) = chat_options.seed() {
			generation_config.insert("seed".into(), seed.into());
		}
		if let Some(thinking_level) = ix_thinking_level(chat_options.reasoning_effort()) {
			generation_config.insert("thinking_level".into(), thinking_level.into());
		}
		// Thought summaries are opt-in; without this the `thought` steps carry a signature only and
		// `reasoning_content` comes back empty. Both reasoning-related options express "I want the
		// model's reasoning text back", so either one turns summaries on.
		if chat_options.capture_reasoning_content() == Some(true)
			|| chat_options.normalize_reasoning_content() == Some(true)
		{
			generation_config.insert("thinking_summaries".into(), "auto".into());
		}
		if let Some(tool_choice) = ix_tool_choice(chat_options.tool_choice()) {
			generation_config.insert("tool_choice".into(), tool_choice);
		}
		// NOTE: `temperature` / `top_p` are not in the documented `generation_config` schema, but
		//       the Interactions overview prose lists temperature among its members. Sent as-is;
		//       the provider ignores or rejects them, rather than genai silently dropping them.
		if let Some(temperature) = chat_options.temperature() {
			generation_config.insert("temperature".into(), temperature.into());
		}
		if let Some(top_p) = chat_options.top_p() {
			generation_config.insert("top_p".into(), top_p.into());
		}

		if !generation_config.is_empty() {
			payload.x_insert("generation_config", generation_config)?;
		}

		// -- Provider-specific payload extension
		// Merged last so callers can intentionally override previously set fields.
		// Deep merge (see `deep_merge`): this is how `generation_config.transcription_config` and
		// every other Interactions-only knob is reached.
		if let Some(extra_body) = chat_options.extra_body() {
			deep_merge(&mut payload, extra_body.clone());
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { body, .. } = web_response;

		let captured_raw_body = options_set.capture_raw_body().unwrap_or_default().then(|| body.clone());

		let interaction: IxInteraction = serde_json::from_value(body)?;

		// -- Capture the provider_model_iden
		let provider_model_iden = match interaction.model.as_deref() {
			Some(model_name) => model_iden.from_name(model_name),
			None => model_iden.clone(),
		};

		if let Some(error_message) = interaction.error_message() {
			tracing::warn!("GeminiIx - interaction reported errors: {error_message}");
		}

		// -- Capture the usage
		let usage = interaction.usage.map(Usage::from).unwrap_or_default();

		// -- Walk the step timeline into content parts
		let mut parts: Vec<ContentPart> = Vec::new();
		let mut reasoning = String::new();
		for step in interaction.steps {
			parts.extend(step.into_content_parts(&mut reasoning));
		}

		// Mirror the Gemini `generateContent` adapter: also hang the signatures off the first tool
		// call. `ChatResponse::into_tool_calls()` drops the standalone `ThoughtSignature` parts, and
		// the API rejects a replayed `function_call` that is not preceded by its `thought` step.
		let signatures: Vec<String> = parts
			.iter()
			.filter_map(|part| match part {
				ContentPart::ThoughtSignature(signature) => Some(signature.clone()),
				_ => None,
			})
			.collect();
		if !signatures.is_empty()
			&& let Some(ContentPart::ToolCall(first_call)) =
				parts.iter_mut().find(|part| matches!(part, ContentPart::ToolCall(_)))
		{
			first_call.thought_signatures = Some(signatures);
		}

		let reasoning_content = (!reasoning.is_empty()).then_some(reasoning);

		Ok(ChatResponse {
			content: MessageContent::from_parts(parts),
			reasoning_content,
			model_iden,
			provider_model_iden,
			stop_reason: ix_status_to_stop_reason(interaction.status),
			usage,
			captured_raw_body,
			response_id: interaction.id,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSourceStream::new(reqwest_builder);

		let ix_stream = GeminiIxStreamer::new(event_source, model_iden.clone(), options_set);
		let frame_tap = ix_stream.frame_tap();
		let chat_stream = ChatStream::from_inter_stream(ix_stream).with_frame_tap(frame_tap);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::GeminiIx,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::GeminiIx,
			feature: "embeddings".to_string(),
		})
	}
}

/// Support functions
impl GeminiIxAdapter {
	fn into_ix_input_steps(
		model_iden: &ModelIden,
		messages: Vec<ChatMessage>,
		systems: &mut Vec<String>,
	) -> Result<Vec<Value>> {
		let mut steps: Vec<Value> = Vec::new();
		let mut emitted_signatures: HashSet<String> = HashSet::new();
		let mut tool_call_names: HashMap<String, String> = HashMap::new();

		for msg in messages {
			match msg.role {
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						systems.push(content);
					}
				}

				ChatRole::User | ChatRole::Tool => {
					let mut content: Vec<Value> = Vec::new();

					for part in msg.content {
						match part {
							ContentPart::Text(text) => content.push(json!({"type": "text", "text": text})),
							ContentPart::Binary(binary) => content.push(binary_to_ix_content(binary)),
							ContentPart::ToolResponse(tool_response) => {
								flush_content_step(&mut steps, &mut content, "user_input");
								let fn_name = ix_function_result_name(model_iden, &tool_response, &tool_call_names)?;
								steps.push(json!({
									"type": "function_result",
									"call_id": tool_response.call_id,
									"name": fn_name,
									"result": [{"type": "text", "text": tool_response.content}],
								}));
							}
							// A user turn carries no tool calls, thoughts or reasoning text.
							ContentPart::ToolCall(_)
							| ContentPart::ThoughtSignature(_)
							| ContentPart::ReasoningContent(_)
							| ContentPart::Custom(_) => (),
						}
					}

					flush_content_step(&mut steps, &mut content, "user_input");
				}

				ChatRole::Assistant => {
					let mut content: Vec<Value> = Vec::new();
					let mut turn_has_thought = false;

					for part in msg.content {
						match part {
							ContentPart::Text(text) => content.push(json!({"type": "text", "text": text})),
							ContentPart::Binary(binary) => content.push(binary_to_ix_content(binary)),
							ContentPart::ThoughtSignature(signature) => {
								flush_content_step(&mut steps, &mut content, "model_output");
								if emitted_signatures.insert(signature.clone()) {
									steps.push(json!({"type": "thought", "signature": signature}));
									turn_has_thought = true;
								}
							}
							ContentPart::ToolCall(tool_call) => {
								flush_content_step(&mut steps, &mut content, "model_output");

								for signature in tool_call.thought_signatures.into_iter().flatten() {
									if emitted_signatures.insert(signature.clone()) {
										steps.push(json!({"type": "thought", "signature": signature}));
										turn_has_thought = true;
									}
								}

								// Needed for server side tools (search, urlContext, etc.,)
								if !turn_has_thought {
									steps.push(json!({
										"type": "thought",
										"signature": SKIP_THOUGHT_SIGNATURE_VALIDATOR,
									}));
									turn_has_thought = true;
								}

								tool_call_names.insert(tool_call.call_id.clone(), tool_call.fn_name.clone());
								steps.push(json!({
									"type": "function_call",
									"id": tool_call.call_id,
									"name": tool_call.fn_name,
									"arguments": tool_call.fn_arguments,
								}));
							}
							ContentPart::ReasoningContent(_)
							| ContentPart::ToolResponse(_)
							| ContentPart::Custom(_) => (),
						}
					}

					flush_content_step(&mut steps, &mut content, "model_output");
				}
			}
		}

		Ok(steps)
	}

	/// Builds the interaction-specific `tools` array.
	/// DOC: <https://ai.google.dev/api/interactions#Resource:Tool>
	fn into_ix_tools(tools: Vec<Tool>) -> Vec<Value> {
		tools
			.into_iter()
			.map(|tool| {
				// -- Server-side tools are a bare type tag
				let builtin_name = match &tool.name {
					ToolName::WebSearch => Some("google_search"),
					ToolName::Custom(name) => IX_BUILTIN_TOOL_NAMES.iter().find(|builtin| *builtin == name).copied(),
				};
				if let Some(builtin_name) = builtin_name {
					return json!({"type": builtin_name});
				}

				// -- Otherwise a flat function declaration (no `functionDeclarations` nesting here)
				let mut function = json!({
					"type": "function",
					"name": tool.name.as_str(),
				});
				if let Some(description) = tool.description {
					let _ = function.x_insert("description", description);
				}
				if let Some(schema) = tool.schema {
					let _ = function.x_insert("parameters", schema);
				}
				function
			})
			.collect()
	}

	fn into_ix_response_format(response_format: Option<&ChatResponseFormat>) -> Option<Value> {
		match response_format? {
			ChatResponseFormat::JsonMode => Some(json!({
				"type": "text",
				"mime_type": "application/json",
			})),
			ChatResponseFormat::JsonSpec(json_spec) => Some(json!({
				"type": "text",
				"mime_type": "application/json",
				"schema": json_spec.schema,
			})),
		}
	}
}

/// Pushes the buffered content blocks as one step of `step_type`, and clears the buffer.
fn flush_content_step(steps: &mut Vec<Value>, content: &mut Vec<Value>, step_type: &str) {
	if content.is_empty() {
		return;
	}
	steps.push(json!({
		"type": step_type,
		"content": std::mem::take(content),
	}));
}

/// Resolves the `name` for a `function_result` step.
///
/// IMPORTANT: The API reference marks `FunctionResultStep.name` optional, but the provider requires
/// it *and* requires it to be the real tool name. Verified against the live API 2026-08-31:
/// correct name → 200, omitted → 400, any other value → 400.
fn ix_function_result_name(
	model_iden: &ModelIden,
	tool_response: &ToolResponse,
	tool_call_names: &HashMap<String, String>,
) -> Result<String> {
	if let Some(fn_name) = tool_response.fn_name.clone() {
		return Ok(fn_name);
	}
	if let Some(fn_name) = tool_call_names.get(&tool_response.call_id).cloned() {
		return Ok(fn_name);
	}

	Err(Error::Internal(format!(
		"{model_iden} - ToolResponse for call_id '{}' has no fn_name, and the matching function_call is not in \
		 this request (it is held server-side by previous_interaction_id). The Gemini Interactions API requires \
		 the tool name on a function_result. Build the response with \
		 `ToolResponse::from_tool_call(&tool_call, ..)`, or set it with `.with_fn_name(..)`.",
		tool_response.call_id
	)))
}

/// Maps a `Binary` onto the matching content block, keyed by MIME type.
/// DOC: <https://ai.google.dev/api/interactions#Resource:Content>
fn binary_to_ix_content(binary: Binary) -> Value {
	let Binary {
		content_type, source, ..
	} = binary;

	let content_kind = if content_type.starts_with("image/") {
		"image"
	} else if content_type.starts_with("audio/") {
		"audio"
	} else if content_type.starts_with("video/") {
		"video"
	} else {
		// `application/pdf` and `text/csv` are the documented document types.
		"document"
	};

	match source {
		BinarySource::Base64(data) => json!({
			"type": content_kind,
			"mime_type": content_type,
			"data": data.as_ref(),
		}),
		BinarySource::Url(uri) => json!({
			"type": content_kind,
			"mime_type": content_type,
			"uri": uri,
		}),
	}
}

// region:    --- Tests

#[cfg(test)]
#[path = "adapter_impl_tests.rs"]
mod tests;

// endregion: --- Tests
