use crate::adapter::adapters::gemini::GeminiStreamer;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream,
	ChatStreamResponse, CompletionTokensDetails, ContentPart, MessageContent, PromptTokensDetails, ReasoningEffort,
	StopReason, Tool, ToolCall, ToolChoice, ToolConfig, ToolName, ToolResponse, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::StreamRequest as RequestBuilder;
use crate::webc::{EventSourceStream, WebClient, WebResponse};
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use value_ext::JsonValueExt;

const SKIP_THOUGHT_SIGNATURE_VALIDATOR: &str = "skip_thought_signature_validator";

pub struct GeminiAdapter;

// Per gemini doc (https://x.com/jeremychone/status/1916501987371438372)
pub(in crate::adapter) const REASONING_LOW: u32 = 1000;
pub(in crate::adapter) const REASONING_MEDIUM: u32 = 8000;
pub(in crate::adapter) const REASONING_HIGH: u32 = 24000;

/// Important
/// - For now Low and Minimal are the same for gemini
fn insert_gemini_thinking_budget_value(payload: &mut Value, effort: &ReasoningEffort) -> Result<()> {
	// -- for now, match minimal to Low (because zero is not supported by 2.5 pro)
	let budget = match effort {
		ReasoningEffort::Zero => None,
		ReasoningEffort::Low | ReasoningEffort::Minimal => Some(REASONING_LOW),
		ReasoningEffort::Medium => Some(REASONING_MEDIUM),
		ReasoningEffort::High | ReasoningEffort::Max | ReasoningEffort::XHigh => Some(REASONING_HIGH),
		ReasoningEffort::Budget(budget) => Some(*budget),
	};

	if let Some(budget) = budget {
		payload.x_insert("/generationConfig/thinkingConfig/thinkingBudget", budget)?;
	}
	Ok(())
}

const GEMINI_BUILTIN_TOOL_NAMES: &[&str] = &["googleSearch", "googleSearchRetrieval", "codeExecution", "urlContext"];

// The flag tool_config.include_server_side_tool_invocations is required for
// the Gemini API and ignored by the vertex endpoint
fn needs_server_side_tool_invocations(tools: &[Value]) -> bool {
	let has_builtin = tools.iter().any(|tool| {
		tool.as_object()
			.is_some_and(|obj| obj.keys().any(|key| GEMINI_BUILTIN_TOOL_NAMES.contains(&key.as_str())))
	});
	let has_functions = tools.iter().any(|tool| tool.get("functionDeclarations").is_some());

	has_builtin && has_functions
}

fn gemini_tool_config(tool_choice: Option<&ToolChoice>) -> Option<Value> {
	let tool_choice = tool_choice?;
	let mut config = json!({
		"functionCallingConfig": {
			"mode": match tool_choice {
				ToolChoice::Auto => "AUTO",
				ToolChoice::None => "NONE",
				ToolChoice::Required | ToolChoice::Tool { .. } => "ANY",
			}
		}
	});
	if let Some(name) = tool_choice.tool_name() {
		config["functionCallingConfig"]["allowedFunctionNames"] = json!([name]);
	}
	Some(config)
}

// curl \
//   -H 'Content-Type: application/json' \
//   -d '{"contents":[{"parts":[{"text":"Explain how AI works"}]}]}' \
//   -X POST 'https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'

impl GeminiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GEMINI_API_KEY";
}

impl Adapter for GeminiAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	async fn all_model_names(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		// -- url
		let base_url = endpoint.base_url();
		let url = format!("{base_url}models");

		// -- auth / headers
		let api_key = auth.single_key_value().ok();
		let headers = api_key
			.map(|api_key| Headers::from(("x-goog-api-key".to_string(), api_key)))
			.unwrap_or_default();

		// -- Exec request
		let mut res = web_client
			.do_get(&url, &headers)
			.await
			.map_err(|webc_error| Error::WebAdapterCall {
				adapter_kind: kind,
				webc_error,
			})?;

		// -- Format result
		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("name")?;
				// Gemini model names are usually prefixed with "models/"
				let model_name = model_name.strip_prefix("models/").unwrap_or(&model_name).to_string();
				models.push(model_name);
			}
		}

		Ok(models)
	}

	/// NOTE: As Google Gemini has decided to put their API_KEY in the URL,
	///       this will return the URL without the API_KEY in it. The API_KEY will need to be added by the caller.
	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let (_, model_name) = model.model_name.namespace_and_name();
		let url = match service_type {
			ServiceType::Chat => format!("{base_url}models/{model_name}:generateContent"),
			ServiceType::ChatStream => format!("{base_url}models/{model_name}:streamGenerateContent?alt=sse"),
			ServiceType::Embed => format!("{base_url}models/{model_name}:embedContent"), // Gemini embeddings API
		};
		Ok(url)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;
		let (_, model_name) = model.model_name.namespace_and_name();

		// -- api_key
		let api_key = get_api_key(auth, &model)?;
		let headers = Headers::from(("x-goog-api-key".to_string(), api_key.to_string()));

		let (payload, provider_model_name) =
			Self::build_gemini_request_payload(&model, model_name, chat_req, options_set)?;

		// -- url
		let provider_model = model.from_name(&provider_model_name);
		let url = Self::get_service_url(&provider_model, service_type, endpoint)?;

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the provider_model_iden
		// TODO: Need to be implemented (if available), for now, just clone model_iden
		let provider_model_name: Option<String> = body.x_remove("modelVersion").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);
		let gemini_response = Self::body_to_gemini_chat_response(&model_iden.clone(), body)?;
		let GeminiChatResponse {
			content: gemini_content,
			usage,
			stop_reason,
		} = gemini_response;
		let stop_reason = stop_reason.map(StopReason::from);

		// Parts keep the order the API returned them in, and a thought signature sits
		// immediately before the part it arrived on (a call also mirrors its own in
		// `thought_signatures`). Consecutive text parts merge into one; thought summaries
		// go to `reasoning_content`.
		let mut parts: Vec<ContentPart> = Vec::new();
		let mut reasoning_text = String::new();
		let mut text_buffer = String::new();

		fn flush_text(text_buffer: &mut String, parts: &mut Vec<ContentPart>) {
			if !text_buffer.is_empty() {
				parts.push(ContentPart::Text(std::mem::take(text_buffer)));
			}
		}

		for GeminiPart {
			data,
			thought_signature,
		} in gemini_content
		{
			if let GeminiPartData::Reasoning(reasoning) = &data {
				reasoning_text.push_str(reasoning);
			}
			if let Some(signature) = thought_signature {
				flush_text(&mut text_buffer, &mut parts);
				parts.push(ContentPart::ThoughtSignature(signature));
			}
			match data {
				GeminiPartData::Text(text) => text_buffer.push_str(&text),
				GeminiPartData::ToolCall(tool_call) => {
					flush_text(&mut text_buffer, &mut parts);
					parts.push(ContentPart::ToolCall(tool_call));
				}
				GeminiPartData::Binary(binary) => {
					flush_text(&mut text_buffer, &mut parts);
					parts.push(ContentPart::Binary(binary));
				}
				GeminiPartData::Reasoning(_) => {}
			}
		}
		flush_text(&mut text_buffer, &mut parts);
		let content = MessageContent::from_parts(parts);

		Ok(ChatResponse {
			content,
			reasoning_content: (!reasoning_text.is_empty()).then_some(reasoning_text),
			model_iden,
			provider_model_iden,
			stop_reason,
			usage,
			captured_raw_body: None, // Set by the client exec_chat
			response_id: None,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSourceStream::new(reqwest_builder);

		let gemini_stream = GeminiStreamer::new(event_source, model_iden.clone(), options_set);
		let frame_tap = gemini_stream.frame_tap();
		let chat_stream = ChatStream::from_inter_stream(gemini_stream).with_frame_tap(frame_tap);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		super::embed::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		super::embed::to_embed_response(model_iden, web_response, options_set)
	}
}

// region:    --- Support

/// Support functions for GeminiAdapter
impl GeminiAdapter {
	pub(in crate::adapter) fn body_to_gemini_chat_response(
		model_iden: &ModelIden,
		mut body: Value,
	) -> Result<GeminiChatResponse> {
		// If the body has an `error` property, then it is assumed to be an error.
		if body.get("error").is_some() {
			return Err(Error::ChatResponse {
				model_iden: model_iden.clone(),
				body,
			});
		}

		let mut content: Vec<GeminiPart> = Vec::new();

		// Extract usage before content/parts so it is available even in
		// usage-only tail frames (finishReason + usageMetadata but no content).
		let usage = body.x_take::<Value>("usageMetadata").map(Self::into_usage).unwrap_or_default();

		// -- Read multipart
		let parts = match body.x_take::<Vec<Value>>("/candidates/0/content/parts") {
			Ok(parts) => parts,
			Err(_) => {
				let finish_reason = body
					.x_remove::<String>("/candidates/0/finishReason")
					.ok()
					.or_else(|| body.x_remove::<String>("/candidates/finishReason").ok());
				let saw_usage_only_tail = body.get("candidates").is_some() || finish_reason.is_some();

				// Gemini streaming sends a final frame with finishReason + usageMetadata
				// but no content.parts. This is normal — return Ok with empty content.
				if saw_usage_only_tail {
					return Ok(GeminiChatResponse {
						content,
						usage,
						stop_reason: finish_reason,
					});
				}

				let body = json!({
					"finishReason": finish_reason,
					"usageMetadata": Value::Null,
				});
				return Err(Error::ChatResponse {
					model_iden: model_iden.clone(),
					body,
				});
			}
		};

		let mut tool_call_counter: usize = 0;
		for mut part in parts {
			let thought_signature = take_string(&mut part, "thoughtSignature");
			let is_thought = take_bool(&mut part, "thought");

			let data = if let Ok(fc) = part.x_take::<Value>("functionCall") {
				let fn_name: String = fc.x_get("name").unwrap_or_default();
				// Gemini omits call_id; synthesize a unique one to avoid
				// collisions when the same tool is called multiple times.
				let call_id: String = fc
					.x_get("id")
					.unwrap_or_else(|_| format!("call#{}#{}", fn_name, tool_call_counter));
				tool_call_counter += 1;
				GeminiPartData::ToolCall(ToolCall {
					call_id,
					fn_name,
					fn_arguments: fc.x_get("args").unwrap_or(Value::Null),
					thought_signatures: thought_signature.clone().map(|signature| vec![signature]),
				})
			} else if let Some(text) = take_string(&mut part, "text") {
				if is_thought {
					GeminiPartData::Reasoning(text)
				} else {
					GeminiPartData::Text(text)
				}
			} else if let Ok(inline_data) = part.x_take::<Value>("inlineData")
				&& let Ok(mime_type) = inline_data.x_get::<String>("mimeType")
				&& let Ok(data) = inline_data.x_get::<String>("data")
			{
				// Note: Gemini may send inline data in multiple parts, but for now,
				// we will treat each part as a separate binary content.
				// We can consider concatenating them if needed in the future.
				GeminiPartData::Binary(Binary::from_base64(mime_type, data, None))
			} else if thought_signature.is_some() {
				// A signature-only part (seen as an empty-text part in streams).
				GeminiPartData::Text(String::new())
			} else {
				continue;
			};

			content.push(GeminiPart {
				data,
				thought_signature,
			});
		}
		let stop_reason: Option<String> = body.x_take("/candidates/0/finishReason").ok();

		Ok(GeminiChatResponse {
			content,
			usage,
			stop_reason,
		})
	}

	/// Builds the Gemini JSON payload from a ChatRequest, including reasoning budget
	/// resolution, system instruction, tools, response format, and chat options.
	/// Returns (payload, provider_model_name) where provider_model_name may differ
	/// from model_name if a reasoning suffix was stripped.
	pub(in crate::adapter) fn build_gemini_request_payload(
		model: &ModelIden,
		model_name: &str,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<(Value, String)> {
		// -- Reasoning Budget
		let (provider_model_name, computed_reasoning_effort) = match (model_name, options_set.reasoning_effort()) {
			// No explicit reasoning_effort, try to infer from model name suffix (supports -zero)
			(model, None) => {
				if let Some((prefix, last)) = model_name.rsplit_once('-') {
					let reasoning = match last {
						// 'zero' is canonical ('none' for backward compatibility)
						"none" | "zero" => Some(ReasoningEffort::Budget(0)),
						"low" | "minimal" => Some(ReasoningEffort::Low),
						"medium" => Some(ReasoningEffort::Medium),
						"high" => Some(ReasoningEffort::High),
						"xhigh" => Some(ReasoningEffort::XHigh),
						"max" => Some(ReasoningEffort::Max),
						_ => None,
					};
					// strip the reasoning suffix from the model name if one was matched
					let model = if reasoning.is_some() { prefix } else { model };
					(model, reasoning)
				} else {
					(model, None)
				}
			}
			// TOOD: make it more elegant
			(model, Some(effort)) => (model, Some(effort.clone())),
		};

		// -- parts
		let GeminiChatRequestParts {
			system,
			contents,
			tools,
		} = Self::into_gemini_request_parts(model, chat_req)?;

		let mut payload = json!({});

		// -- Set the reasoning effort
		if let Some(computed_reasoning_effort) = computed_reasoning_effort {
			// -- For gemini-3, gemma-4 use the thinkingLevel
			let models = ["gemini-3", "gemma-4"];
			if models.iter().any(|m| provider_model_name.contains(m)) {
				let thinking_level = match computed_reasoning_effort {
					ReasoningEffort::Zero => None,
					ReasoningEffort::Budget(_) => None,
					ReasoningEffort::Minimal => Some("MINIMAL"),
					ReasoningEffort::Low => Some("LOW"),
					ReasoningEffort::Medium => Some("MEDIUM"),
					ReasoningEffort::High | ReasoningEffort::Max | ReasoningEffort::XHigh => Some("HIGH"),
				};
				if let Some(thinking_level) = thinking_level {
					payload.x_insert("/generationConfig/thinkingConfig/thinkingLevel", thinking_level)?;
				}
			}
			// -- Otherwise, Do thinking budget
			else {
				insert_gemini_thinking_budget_value(&mut payload, &computed_reasoning_effort)?;
			}
		}

		// -- Opt-in for includeThoughts: only request thought content when
		// the caller explicitly asks for reasoning content capture.
		// Thought *signatures* are always returned by the API regardless of this flag.
		if options_set.capture_reasoning_content() == Some(true) {
			payload.x_insert("/generationConfig/thinkingConfig/includeThoughts", true)?;
		}

		// Note: It's unclear from the spec if the content of systemInstruction should have a role.
		//       Right now, it is omitted (since the spec states it can only be "user" or "model")
		//       It seems to work. https://ai.google.dev/api/rest/v1beta/models/generateContent
		if let Some(system) = system {
			payload.x_insert("systemInstruction", json!({ "parts": [{ "text": system }] }))?;
		}

		// `include_server_side_tool_invocations` is set automatically when the request mixes
		// a builtin (googleSearch, urlContext, codeExecution, ...) with function declarations.
		// See https://ai.google.dev/gemini-api/docs/generate-content/tool-combination
		let server_side_invocations = tools.as_deref().is_some_and(needs_server_side_tool_invocations);
		if let Some(tools) = tools {
			payload.x_insert("tools", tools)?;
		}

		let tool_config = gemini_tool_config(options_set.tool_choice());
		if tool_config.is_some() || server_side_invocations {
			// Merged rather than overwritten: `tool_choice` may already have put a
			// `functionCallingConfig` here, and both belong under the same `toolConfig`.
			let mut tool_config = tool_config.unwrap_or_else(|| json!({}));
			if server_side_invocations {
				tool_config.x_insert("includeServerSideToolInvocations", true)?;
			}
			payload.x_insert("toolConfig", tool_config)?;
		}

		// -- Contents (messages, after tools)
		payload.x_insert("contents", contents)?;

		// -- Response Format
		if let Some(ChatResponseFormat::JsonSpec(st_json)) = options_set.response_format() {
			payload.x_insert("/generationConfig/responseMimeType", "application/json")?;
			// Send the JSON Schema as-is to `responseJsonSchema`, Gemini's JSON
			// Schema-native structured-output field (Gemini 2.5+). It accepts full
			// JSON Schema — `$ref`/`$defs`, `anyOf`, `type: "null"`, etc. — so no
			// conversion to the restricted OpenAPI `responseSchema` shape is needed.
			payload.x_insert("/generationConfig/responseJsonSchema", st_json.schema.clone())?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("/generationConfig/temperature", temperature)?;
		}
		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("/generationConfig/stopSequences", options_set.stop_sequences())?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("/generationConfig/maxOutputTokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("/generationConfig/topP", top_p)?;
		}
		if let Some(seed) = options_set.seed() {
			payload.x_insert("/generationConfig/seed", seed)?;
		}
		if let Some(extra_body) = options_set.extra_body() {
			payload.x_merge(extra_body.clone())?;
		}

		Ok((payload, provider_model_name.to_string()))
	}

	/// See gemini doc: https://ai.google.dev/api/generate-content#UsageMetadata
	pub(in crate::adapter) fn into_usage(mut usage_value: Value) -> Usage {
		let total_tokens: Option<i32> = usage_value.x_take("totalTokenCount").ok();

		// -- Compute prompt tokens
		let prompt_tokens: Option<i32> = usage_value.x_take("promptTokenCount").ok();
		// Note: https://developers.googleblog.com/en/gemini-2-5-models-now-support-implicit-caching/
		//       It does say `cached_content_token_count`, but in the json, it's probably
		//       `cachedContenTokenCount` (Could not verify for implicit cache, did not see it yet)
		// Note: It seems the promptTokenCount is inclusive of the cachedContentTokenCount
		//       see: https://ai.google.dev/gemini-api/docs/caching?lang=python#generate-content
		//       (this was for explicit caching, but should be the same for implicit)
		//       ```
		//       prompt_token_count: 696219
		//       cached_content_token_count: 696190
		//       candidates_token_count: 214
		//       total_token_count: 696433
		//       ```
		//       So, in short same as Open asi
		let g_cached_tokens: Option<i32> = usage_value.x_take("cachedContentTokenCount").ok();

		// When using urlContext or search, Gemini sticks those token counts into
		// a separate field called toolUsePromptTokenCount. We are just adding that to the input prompt
		let g_tool_use_tokens: Option<i32> = usage_value.x_take("toolUsePromptTokenCount").ok();
		let prompt_tokens = match (prompt_tokens, g_tool_use_tokens) {
			(Some(prompt), Some(tool_use)) => Some(prompt + tool_use),
			(None, Some(tool_use)) => Some(tool_use),
			(prompt, None) => prompt,
		};

		let prompt_tokens_details = g_cached_tokens.map(|g_cached_tokens| PromptTokensDetails {
			cache_creation_tokens: None,
			cache_creation_details: None,
			cached_tokens: Some(g_cached_tokens),
			audio_tokens: None,
		});

		// -- Compute completion tokens
		let g_candidate_tokens: Option<i32> = usage_value.x_take("candidatesTokenCount").ok();
		let g_thoughts_tokens: Option<i32> = usage_value.x_take("thoughtsTokenCount").ok();
		// IMPORTANT: For Gemini, the `thoughtsTokenCount` (~reasoning_tokens) is not included
		//            in the root `candidatesTokenCount` (~completion_tokens).
		//            Therefore, some computation is needed to normalize it in the "OpenAI API Way,"
		//            meaning `completion_tokens` represents the total of completion tokens,
		//            and the details provide a breakdown of the specific components.
		let (completion_tokens, completion_tokens_details) = match (g_candidate_tokens, g_thoughts_tokens) {
			(Some(c_tokens), Some(t_tokens)) => (
				Some(c_tokens + t_tokens),
				Some(CompletionTokensDetails {
					accepted_prediction_tokens: None,
					rejected_prediction_tokens: None,
					reasoning_tokens: Some(t_tokens),
					audio_tokens: None,
				}),
			),
			(None, Some(t_tokens)) => {
				(
					None,
					Some(CompletionTokensDetails {
						accepted_prediction_tokens: None,
						rejected_prediction_tokens: None,
						reasoning_tokens: Some(t_tokens), // should be safe enough
						audio_tokens: None,
					}),
				)
			}
			(c_tokens, None) => (c_tokens, None),
		};

		Usage {
			prompt_tokens,
			// for now, None for Gemini
			prompt_tokens_details,

			completion_tokens,

			completion_tokens_details,

			total_tokens,
		}
	}

	/// Takes the genai ChatMessages and builds the System string and JSON Messages for Gemini.
	/// - Role mapping `ChatRole:User -> role: "user"`, `ChatRole::Assistant -> role: "model"`
	/// - `ChatRole::System` is concatenated (with an empty line) into a single `system` for the system instruction.
	///   - This adapter uses version v1beta, which supports `systemInstruction`
	/// - The eventual `chat_req.system` is pushed first into the "systemInstruction"
	pub(in crate::adapter) fn into_gemini_request_parts(
		model_iden: &ModelIden, // use for error reporting
		chat_req: ChatRequest,
	) -> Result<GeminiChatRequestParts> {
		let mut contents: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();
		// The name of each call so far, by call id, for the responses that answer it.
		let mut call_names: HashMap<String, String> = HashMap::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build
		for msg in chat_req.messages {
			for call in msg.content.tool_calls() {
				call_names.insert(call.call_id.clone(), call.fn_name.clone());
			}
			match msg.role {
				// For now, system goes as "user" (later, we might have adapter_config.system_to_user_impl)
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						systems.push(content);
					}
				}
				ChatRole::User => {
					let mut parts_values: Vec<Value> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::Text(text) => parts_values.push(json!({"text": text})),
							ContentPart::Binary(binary) => {
								let Binary {
									content_type, source, ..
								} = binary;
								match &source {
									BinarySource::Url(url) => parts_values.push(json!({
										"file_data": {
											"mime_type": content_type,
											"file_uri": url
										}
									})),
									BinarySource::Base64(content) => parts_values.push(json!({
										"inline_data": {
											"mime_type": content_type,
											"data": content
										}
									})),
								}
							}
							ContentPart::ToolCall(tool_call) => {
								parts_values.push(gemini_function_call_part(tool_call)?);
							}
							ContentPart::ToolResponse(tool_response) => {
								parts_values.push(gemini_function_response_part(tool_response, &call_names)?);
							}
							ContentPart::ThoughtSignature(thought) => {
								parts_values.push(json!({
									"thoughtSignature": thought
								}));
							}

							ContentPart::ReasoningContent(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}

					contents.push(json!({"role": "user", "parts": parts_values}));
				}
				ChatRole::Assistant => {
					let is_gemini_3 = model_iden.model_name.contains("gemini-3");
					let parts = GeminiModelPart::from_assistant_content(msg.content, is_gemini_3);
					if !parts.is_empty() {
						contents.push(json!({"role": "model", "parts": serde_json::to_value(parts)?}));
					}
				}
				ChatRole::Tool => {
					let mut parts_values: Vec<Value> = Vec::new();
					for part in msg.content {
						match part {
							ContentPart::ToolCall(tool_call) => {
								parts_values.push(gemini_function_call_part(tool_call)?);
							}
							ContentPart::ToolResponse(tool_response) => {
								parts_values.push(gemini_function_response_part(tool_response, &call_names)?);
							}
							ContentPart::ThoughtSignature(thought) => {
								parts_values.push(json!({
									"thoughtSignature": thought
								}));
							}
							ContentPart::ReasoningContent(_) => {}
							_ => {
								return Err(Error::MessageContentTypeNotSupported {
									model_iden: model_iden.clone(),
									cause: "ChatRole::Tool can only contain ToolCall, ToolResponse, or Thought content parts",
								});
							}
						}
					}

					contents.push(json!({"role": "user", "parts": parts_values}));
				}
			}
		}

		let system = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		// -- Post-process: merge consecutive tool-response "user" entries into a single entry.
		// Gemini FC protocol requires all functionResponse parts to be in one "user" turn
		// following the "model" turn with functionCall parts.
		let contents = Self::merge_consecutive_tool_response_entries(contents);

		// -- Build tools
		let tools = if let Some(req_tools) = chat_req.tools {
			let mut tools: Vec<Value> = Vec::new();
			// Note: This is to add only one function_declarations in the tools as per the gemini spec
			//       The rest are builtins
			let mut function_declarations: Vec<Value> = Vec::new();
			for req_tool in req_tools {
				match Self::tool_to_gemini_tool(req_tool)? {
					GeminiTool::Builtin(value) => tools.push(value),
					GeminiTool::User(value) => function_declarations.push(value),
				}
			}
			if !function_declarations.is_empty() {
				tools.push(json!({"functionDeclarations": function_declarations}));
			}
			Some(tools)
		} else {
			None
		};

		Ok(GeminiChatRequestParts {
			system,
			contents,
			tools,
		})
	}

	fn tool_to_gemini_tool(tool: Tool) -> Result<GeminiTool> {
		let Tool {
			name,
			description,
			schema,
			config,
			..
		} = tool;

		// Built-in WebSearch for Gemini
		let name_str = match &name {
			ToolName::WebSearch => "googleSearch",
			ToolName::Custom(name) => name.as_str(),
		};

		// -- if it is a builtin tool
		if GEMINI_BUILTIN_TOOL_NAMES.contains(&name_str) {
			let config = match config {
				// GoogleSearch does not take any config for now
				Some(ToolConfig::WebSearch(_config)) => Some(json!({})),
				// If custom, user knows better
				Some(ToolConfig::Custom(config)) => Some(config),
				// For now, none is empty
				None => None,
			};
			Ok(GeminiTool::Builtin(json!({ name_str: config })))
		}
		// -- otherwise, user tool
		else {
			let parameters = schema.filter(|s| !s.is_null());
			// Use `parametersJsonSchema`, Gemini's JSON Schema-native function-
			// declaration field (Gemini 2.5+), and send the schema as-is. Unlike
			// the restricted `parameters` (OpenAPI Schema) field — which rejects
			// `$ref` with `Unknown name "$ref"` — `parametersJsonSchema` accepts
			// full JSON Schema, so no `$ref` inlining / OpenAPI conversion is
			// needed.
			Ok(GeminiTool::User(json!({
				"name": name_str,
				"description": description,
				"parametersJsonSchema": parameters,
			})))
		}
	}
}

impl GeminiAdapter {
	/// Merge consecutive "user" entries that contain only functionResponse parts
	/// into a single entry. Gemini requires all function responses in one turn.
	fn merge_consecutive_tool_response_entries(contents: Vec<Value>) -> Vec<Value> {
		fn is_tool_response_entry(entry: &Value) -> bool {
			if entry.get("role").and_then(|r| r.as_str()) != Some("user") {
				return false;
			}
			if let Some(parts) = entry.get("parts").and_then(|p| p.as_array()) {
				!parts.is_empty() && parts.iter().all(|p| p.get("functionResponse").is_some())
			} else {
				false
			}
		}

		let mut result: Vec<Value> = Vec::with_capacity(contents.len());
		for entry in contents {
			if is_tool_response_entry(&entry) {
				// Check if previous entry is also a tool response — merge
				if let Some(prev) = result.last_mut()
					&& is_tool_response_entry(prev)
					&& let (Some(prev_parts), Some(new_parts)) = (
						prev.get_mut("parts").and_then(|p| p.as_array_mut()),
						entry.get("parts").and_then(|p| p.as_array()),
					) {
					prev_parts.extend(new_parts.iter().cloned());
					continue;
				}
			}
			result.push(entry);
		}
		result
	}
}

pub enum GeminiTool {
	Builtin(Value),
	User(Value),
}

pub(in crate::adapter) struct GeminiChatResponse {
	pub content: Vec<GeminiPart>,
	pub usage: Usage,
	pub stop_reason: Option<String>,
}

pub(in crate::adapter) struct GeminiPart {
	pub data: GeminiPartData,
	pub thought_signature: Option<String>,
}

pub(in crate::adapter) enum GeminiPartData {
	Text(String),
	Binary(Binary),
	/// Also mirrors its own signature in `ToolCall.thought_signatures`.
	ToolCall(ToolCall),
	Reasoning(String),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiModelPart {
	#[serde(flatten)]
	data: GeminiModelPartData,
	#[serde(skip_serializing_if = "Option::is_none")]
	thought_signature: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum GeminiModelPartData {
	Text(String),
	FunctionCall {
		#[serde(skip_serializing_if = "Option::is_none")]
		id: Option<String>,
		name: String,
		args: Value,
	},
}

impl GeminiModelPart {
	fn from_assistant_content(content: MessageContent, is_gemini_3: bool) -> Vec<Self> {
		let call_signatures: HashSet<String> = content
			.tool_calls()
			.into_iter()
			.filter_map(|call| call.thought_signatures.clone())
			.flatten()
			.collect();

		let mut parts: Vec<Self> = Vec::new();
		let mut pending_signature: Option<String> = None;
		let mut is_first_call = true;

		for part in content {
			match part {
				ContentPart::ThoughtSignature(signature) if call_signatures.contains(&signature) => {}
				ContentPart::ThoughtSignature(signature) => pending_signature = Some(signature),
				ContentPart::Text(text) => parts.push(Self {
					data: GeminiModelPartData::Text(text),
					thought_signature: pending_signature.take(),
				}),
				ContentPart::ToolCall(call) => {
					// For Gemini 3 models, the first call of a turn must carry a signature; when there
					// is none, the API accepts this placeholder.
					// See: https://ai.google.dev/gemini-api/docs/thought-signatures#faqs
					let thought_signature = call
						.thought_signatures
						.and_then(|signatures| signatures.into_iter().next())
						.or_else(|| pending_signature.take())
						.or_else(|| {
							(is_gemini_3 && is_first_call).then(|| SKIP_THOUGHT_SIGNATURE_VALIDATOR.to_string())
						});
					parts.push(Self {
						data: GeminiModelPartData::FunctionCall {
							id: gemini_call_id(&call.call_id).map(str::to_string),
							name: call.fn_name,
							args: call.fn_arguments,
						},
						thought_signature,
					});
					is_first_call = false;
				}
				// Not replayed in a model turn.
				ContentPart::Binary(_)
				| ContentPart::ToolResponse(_)
				| ContentPart::ReasoningContent(_)
				| ContentPart::Custom(_) => {}
			}
		}

		parts
	}
}

pub(in crate::adapter) struct GeminiChatRequestParts {
	pub system: Option<String>,
	/// The chat history (user and assistant, except for the last user message which is a message)
	pub contents: Vec<Value>,

	/// The tools to use
	pub tools: Option<Vec<Value>>,
}

// region:    --- Helpers

/// Extract and remove a string field from a JSON Value.
fn take_string(v: &mut Value, key: &str) -> Option<String> {
	v.as_object_mut()
		.and_then(|m| m.remove(key))
		.and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
}

/// Extract and remove a boolean field from a JSON Value, defaulting to false.
fn take_bool(v: &mut Value, key: &str) -> bool {
	v.as_object_mut()
		.and_then(|m| m.remove(key))
		.and_then(|v| v.as_bool())
		.unwrap_or(false)
}

/// A `functionCall` part, with the call's id when it has one (see `gemini_call_id`).
fn gemini_function_call_part(tool_call: ToolCall) -> Result<Value> {
	let mut function_call = json!({
		"name": tool_call.fn_name,
		"args": tool_call.fn_arguments,
	});
	if let Some(id) = gemini_call_id(&tool_call.call_id) {
		function_call.x_insert("id", id)?;
	}
	Ok(json!({ "functionCall": function_call }))
}

/// A `functionResponse` part, with the id of the call it answers when it has one (see `gemini_call_id`).
///
/// Its name must be the call's: Gemini rejects a response that carries a call's id under another
/// name (400 INVALID_ARGUMENT). The name is the response's `fn_name`, else the name of the call with
/// this id earlier in the request, else the name in a stand-in id. When none is found, the call id
/// stands in for the name and no id is sent.
fn gemini_function_response_part(tool_response: ToolResponse, call_names: &HashMap<String, String>) -> Result<Value> {
	let fn_name = tool_response
		.fn_name
		.or_else(|| call_names.get(&tool_response.call_id).cloned())
		.or_else(|| infer_gemini_synthetic_call_fn_name(&tool_response.call_id));
	let id = call_names
		.contains_key(&tool_response.call_id)
		.then(|| gemini_call_id(&tool_response.call_id))
		.flatten();
	let fn_name = fn_name.as_deref().unwrap_or(&tool_response.call_id);
	let mut function_response = json!({
		"name": fn_name,
		"response": {
			"name": fn_name,
			"content": tool_response.content,
		}
	});
	if let Some(id) = id {
		function_response.x_insert("id", id)?;
	}
	Ok(json!({ "functionResponse": function_response }))
}

/// The id to send back to Gemini for a call: the call's own id (Gemini's, when it returned one),
/// but not the stand-in this adapter makes for a call that came without one (`call#{name}#{n}`),
/// so such calls are sent without an id and matched in order.
/// Ids from other providers or callers are sent as they are.
fn gemini_call_id(call_id: &str) -> Option<&str> {
	(!call_id.is_empty() && infer_gemini_synthetic_call_fn_name(call_id).is_none()).then_some(call_id)
}

fn infer_gemini_synthetic_call_fn_name(call_id: &str) -> Option<String> {
	let rest = call_id.strip_prefix("call#")?;
	let (name, counter) = rest.rsplit_once('#')?;
	if name.is_empty() || counter.parse::<usize>().is_err() {
		return None;
	}
	Some(name.to_string())
}

// endregion: --- Helpers

#[cfg(test)]
#[path = "adapter_impl_tests.rs"]
mod tests;
