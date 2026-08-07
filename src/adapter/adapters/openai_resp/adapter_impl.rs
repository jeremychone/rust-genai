use super::{OpenAIRespStreamer, RespResponse};
use crate::adapter::adapters::openai::OpenAIAdapter;
use crate::adapter::adapters::openai::cache_policy::{
	OpenAiPromptCachePolicy, OpenAiProtocol, is_gpt_5_6_or_later, openai_prompt_cache_policy,
	supports_openai_responses_prompt_cache_options,
};
use crate::adapter::adapters::openai::schema::{
	OpenAiResponseFormatPlan, response_format_plan, tool_parameters_schema,
};
use crate::adapter::adapters::support::{
	TOOL_RESULT_IMAGES_LABEL, assistant_embedded_tool_response_err, get_api_key, tool_response_fallback_text,
};
use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	CacheControl, ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, ContentPart,
	MessageContent, ReasoningEffort, StopReason, Tool, ToolChoice, ToolConfig, ToolName, ToolResponse, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{EventSourceStream, WebClient, WebResponse};
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;
use tracing::warn;
use value_ext::JsonValueExt;

pub struct OpenAIRespAdapter;

fn openai_resp_tool_choice(tool_choice: Option<&ToolChoice>) -> Option<Value> {
	match tool_choice? {
		ToolChoice::Auto => Some(json!("auto")),
		ToolChoice::None => Some(json!("none")),
		ToolChoice::Required => Some(json!("required")),
		ToolChoice::Tool { name } => Some(json!({
			"type": "function",
			"name": name
		})),
	}
}

impl OpenAIRespAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OPENAI_API_KEY";
}

impl Adapter for OpenAIRespAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_auth(_kind: AdapterKind) -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const BASE_URL: &str = "https://api.openai.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	/// Note: Currently returns the common models (see above)
	async fn all_model_names(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		//
		OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth, web_client).await
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		Self::util_get_service_url(model, service_type, endpoint)
	}

	/// OpenAI Doc: https://platform.openai.com/docs/api-reference/responses/create
	///
	/// ## Note related to OpenAI Responses API
	/// - `.store = false` - To maintain consistent behavior with other chat completions, store is set to false
	/// - `.instructions` For now we do not use the top ".instructions" (genai::ChatRequest.system),
	///   but just add this top system as a regular system message.
	/// - `.summary` reasoning summary is opt-in via `ChatOptions.capture_reasoning_content(true)` → `"detailed"`
	///
	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;
		let (_, model_name) = model.model_name.namespace_and_name();
		let adapter_kind = model.adapter_kind;
		let protocol = OpenAiProtocol::Responses;
		let prompt_cache_policy = if supports_openai_responses_prompt_cache_options(&endpoint) {
			openai_prompt_cache_policy(adapter_kind, model_name, &chat_req, &chat_options, protocol)
		} else {
			None
		};
		let response_format_plan = response_format_plan(&chat_options);

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint)?;

		// -- headers
		let headers = Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")));

		let stream = matches!(service_type, ServiceType::ChatStream);

		// -- compute reasoning_effort and eventual trimmed model_name
		// For now, just for openai AdapterKind
		let (reasoning_effort, model_name): (Option<ReasoningEffort>, &str) =
			if matches!(adapter_kind, AdapterKind::OpenAIResp) {
				let (reasoning_effort, model_name) = chat_options
					.reasoning_effort()
					.cloned()
					.map(|v| (Some(v), model_name))
					.unwrap_or_else(|| ReasoningEffort::from_model_name(model_name));

				(reasoning_effort, model_name)
			} else {
				(None, model_name)
			};

		// -- Extract system prompt before consuming chat_req.
		// Use the Responses API `instructions` field instead of an input system message.
		// `instructions` is the canonical way to set system prompt in the Responses API:
		// - It overrides on each call (important for stateful sessions with previous_response_id)
		// - It separates instructions from conversation items
		// - Inline system messages (ChatRole::System in messages) still go to input as-is
		let instructions = chat_req.system.clone();
		let mut chat_req = chat_req;
		chat_req.system = None;

		// -- Extract stateful session fields before consuming chat_req
		let previous_response_id = chat_req.previous_response_id.clone();
		let explicit_store = chat_req.store;

		// -- Build the basic payload
		let OpenAIRespRequestParts {
			input_items: messages,
			tools,
		} = Self::into_openai_request_parts(&model, chat_req, prompt_cache_policy.as_ref())?;

		// Store: always opt-in. If not explicitly set, default is false.
		// Privacy first: we never implicitly set store=true, even when previous_response_id is set.
		// If previous_response_id is set without store=true, log a warning — the caller must be explicit.
		let store = explicit_store.unwrap_or(false);
		if previous_response_id.is_some() && explicit_store != Some(true) {
			tracing::warn!(
				"previous_response_id is set but store is not explicitly true — \
				 stateful session requires store=true to work. Set `store: Some(true)` explicitly."
			);
		}

		let mut payload = json!({
			"store": store,
			"model": model_name,
			"stream": stream,
		});

		if let Some(policy) = prompt_cache_policy.as_ref() {
			let mut prompt_cache_options = json!({"mode": "explicit"});
			if let Some(ttl) = policy.ttl {
				prompt_cache_options.x_insert("ttl", ttl)?;
			}
			payload.x_insert("prompt_cache_options", prompt_cache_options)?;
		}

		// -- System prompt as instructions
		if let Some(instructions) = &instructions {
			payload.x_insert("instructions", instructions.as_str())?;
		}

		// -- Stateful session: add previous_response_id
		if let Some(prev_id) = &previous_response_id {
			payload.x_insert("previous_response_id", prev_id.as_str())?;
		}

		// -- Set reasoning options
		//
		// The `reasoning` object on the request controls two things:
		//   * `.effort` — how much reasoning the model should do
		//   * `.summary` — whether a text summary of the reasoning is
		//     returned in the response (required to populate
		//     `ChatResponse.reasoning_content` for the Responses API)
		//
		// Either half is sufficient to warrant inserting the object;
		// previously the object was only emitted when `reasoning_effort`
		// was set, which silently defeated `capture_reasoning_content(true)`
		// on its own — callers asking for reasoning capture got no
		// `summary=detailed` opt-in, and every response came back with
		// empty `reasoning_content`.
		let capture_reasoning = chat_options.capture_reasoning_content() == Some(true);
		let effort_keyword = reasoning_effort.and_then(|effort| match effort {
			ReasoningEffort::Zero => Some("none"),
			_ => effort.as_keyword(),
		});

		if effort_keyword.is_some() || capture_reasoning {
			let mut reasoning_obj = json!({});
			if let Some(keyword) = effort_keyword {
				reasoning_obj
					.x_insert("effort", keyword)
					.map_err(|e| Error::Internal(format!("reasoning effort insert: {e}")))?;
			}
			if capture_reasoning {
				reasoning_obj
					.x_insert("summary", "detailed")
					.map_err(|e| Error::Internal(format!("reasoning summary insert: {e}")))?;
			}
			payload.x_insert("reasoning", reasoning_obj)?;
		}

		// -- Opt-in: request encrypted reasoning content (thought signatures)
		// when the caller explicitly asks for reasoning content capture.
		if chat_options.capture_reasoning_content() == Some(true) {
			payload.x_insert("include", json!(["reasoning.encrypted_content"]))?;
		}

		// -- Tools (before messages)
		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}
		if let Some(tool_choice) = openai_resp_tool_choice(chat_options.tool_choice()) {
			payload.x_insert("tool_choice", tool_choice)?;
		}

		// -- Messages (after tools)
		payload.x_insert("input", messages)?;

		// -- Compute response format
		let response_format = match response_format_plan {
			OpenAiResponseFormatPlan::None => None,
			OpenAiResponseFormatPlan::JsonMode => Some(json!({"type": "json_object"})),
			OpenAiResponseFormatPlan::JsonSchema { name, schema } => Some(json!({
				"type": "json_schema",
				"name": name,
				"strict": true,
				"schema": schema,
			})),
		};

		// -- Get verbosity
		let verbosity = chat_options.verbosity().and_then(|v| v.as_keyword());

		if response_format.is_some() || verbosity.is_some() {
			let mut value_map = Map::new();
			if let Some(verbosity) = verbosity {
				value_map.insert("verbosity".into(), verbosity.into());
			}
			if let Some(response_format) = response_format {
				value_map.insert("format".into(), response_format);
			}

			payload.x_insert("text", value_map)?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = chat_options.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !chat_options.stop_sequences().is_empty() {
			payload.x_insert("stop", chat_options.stop_sequences())?;
		}

		if let Some(max_tokens) = chat_options.max_tokens() {
			payload.x_insert("max_output_tokens", max_tokens)?;
		}
		if let Some(top_p) = chat_options.top_p() {
			payload.x_insert("top_p", top_p)?;
		}
		if let Some(seed) = chat_options.seed() {
			payload.x_insert("seed", seed)?;
		}

		// -- OpenAI prompt cache options
		if let Some(prompt_cache_key) = chat_options.prompt_cache_key() {
			payload.x_insert("prompt_cache_key", prompt_cache_key)?;
		}
		if !is_gpt_5_6_or_later(model_name)
			&& let Some(cache_control) = chat_options.cache_control()
		{
			let prompt_cache_retention = match cache_control {
				CacheControl::Memory | CacheControl::Ephemeral => Some("in_memory"),
				CacheControl::Ephemeral24h => Some("24h"),
				CacheControl::Ephemeral5m | CacheControl::Ephemeral1h => None,
			};
			if let Some(prompt_cache_retention) = prompt_cache_retention {
				payload.x_insert("prompt_cache_retention", prompt_cache_retention)?;
			}
		}

		// -- Provider-specific payload extension
		// Merged last so callers can intentionally override previously set fields.
		if let Some(extra_body) = chat_options.extra_body() {
			payload.x_merge(extra_body.clone())?;
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

		let resp: RespResponse = serde_json::from_value(body)?;

		// -- Capture the provider_model_iden
		let provider_model_iden = model_iden.from_name(&resp.model);

		// -- Capture the usage
		let usage = resp.usage.map(Usage::from).unwrap_or_default();

		// -- Capture the content
		let mut content: MessageContent = MessageContent::default();
		let reasoning_content: Option<String> = None;

		// -- Extract the content message
		for output_item in resp.output {
			let parts = ContentPart::from_resp_output_item(output_item)?;
			content.extend(parts);
		}

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden,
			provider_model_iden,
			stop_reason: Some(StopReason::from(resp.status)),
			usage,
			captured_raw_body,
			response_id: Some(resp.id),
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_sets: ChatOptionsSet<'_, '_>,
		response_observer: Option<crate::client::BoundResponseObserver>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSourceStream::new(reqwest_builder).with_response_observer(response_observer);
		let openai_stream = OpenAIRespStreamer::new(event_source, model_iden.clone(), options_sets);
		let chat_stream = ChatStream::from_inter_stream(openai_stream);

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
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::OpenAIResp,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::OpenAIResp,
			feature: "embeddings".to_string(),
		})
	}
}

/// Support functions for other adapters that share OpenAI APIs
impl OpenAIRespAdapter {
	pub(in crate::adapter::adapters) fn util_get_service_url(
		_model: &ModelIden,
		service_type: ServiceType,
		// -- utility arguments
		default_endpoint: Endpoint,
	) -> Result<String> {
		let base_url = default_endpoint.base_url();
		// Parse into URL and query-params
		let base_url = reqwest::Url::parse(base_url)
			.map_err(|err| Error::Internal(format!("Cannot parse url: {base_url}. Cause:\n{err}")))?;
		let original_query_params = base_url.query().to_owned();

		let suffix = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => "responses",
			ServiceType::Embed => "embeddings", // TODO: Probably needs to say not supported
		};
		let mut full_url = base_url.join(suffix).map_err(|err| {
			Error::Internal(format!(
				"Cannot joing url suffix '{suffix}' for base_url '{base_url}'. Cause:\n{err}"
			))
		})?;
		full_url.set_query(original_query_params);
		Ok(full_url.to_string())
	}

	/// Takes the genai ChatMessages and builds the OpenAIChatRequestParts
	/// - `genai::ChatRequest.system`, if present, is added as the first message with role 'system'.
	/// - All messages get added with the corresponding roles (tools are not supported for now)
	///
	fn into_openai_request_parts(
		model_iden: &ModelIden,
		chat_req: ChatRequest,
		cache_policy: Option<&OpenAiPromptCachePolicy>,
	) -> Result<OpenAIRespRequestParts> {
		let mut input_items: Vec<Value> = Vec::new();
		let custom_tool_names = chat_req
			.tools
			.as_ref()
			.into_iter()
			.flatten()
			.filter(|tool| tool.custom_format.is_some())
			.map(|tool| tool.name.as_str().to_string())
			.collect::<BTreeSet<_>>();
		let mut custom_call_ids = BTreeSet::new();

		// -- Process the system
		if let Some(system_msg) = chat_req.system {
			input_items.push(json!({"role": "system", "content": system_msg}));
		}

		let mut unamed_file_count = 0;

		// Images rescued from custom tool outputs (`custom_tool_call_output.output` is a
		// raw string on the wire, so it cannot carry `input_image` blocks). They ride in a
		// follow-up `user` message input item. Images from a run of consecutive Tool
		// messages are batched into one trailing item, emitted before the next non-tool
		// message (same batching as the Chat Completions serializer).
		let mut pending_custom_tool_images: Vec<Value> = Vec::new();

		// -- Process the messages
		for msg in chat_req.messages {
			// Index of the tool-image flush item emitted for this iteration (if any), so
			// output items extracted from an embedded `ToolResponse` can be inserted
			// before it, adjacent to the tool-message run they belong to.
			let mut flushed_images_at: Option<usize> = None;
			if !matches!(msg.role, ChatRole::Tool) && !pending_custom_tool_images.is_empty() {
				flushed_images_at = Some(input_items.len());
				input_items.push(tool_images_user_item(std::mem::take(&mut pending_custom_tool_images)));
			}

			let cache_controlled = cache_policy.is_some()
				&& msg
					.options
					.as_ref()
					.and_then(|options| options.cache_control.as_ref())
					.is_some();

			// Note: Will handle more types later
			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						if cache_controlled {
							let mut values = vec![json!({"type": "input_text", "text": content})];
							apply_resp_cache_breakpoint(model_iden, &mut values, "message")?;
							input_items.push(json!({"role": "system", "content": values}));
						} else {
							input_items.push(json!({"role": "system", "content": content}))
						}
					}
					// TODO: Probably need to warn if it is a ToolCalls type of content
				}

				// User - For now support Text and Binary
				ChatRole::User => {
					// -- If we have only text, then, we jjust returned the joined_texts
					if msg.content.is_text_only() && !cache_controlled {
						// NOTE: for now, if no content, just return empty string (respect current logic)
						let content = json!(msg.content.joined_texts().unwrap_or_else(String::new));
						input_items.push(json! ({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();
						// Tool responses embedded in this user message (the Anthropic-style
						// shape where tool results ride as user-message content blocks) are
						// extracted into proper output items (`function_call_output`, or
						// `custom_tool_call_output` for custom tool calls) emitted BEFORE
						// this user message item, translating the conventional
						// assistant-tool_calls -> user-carried-results ordering.
						let mut embedded_output_items: Vec<Value> = Vec::new();

						for part in msg.content {
							match part {
								// -- Simple Text
								ContentPart::Text(content) => {
									values.push(json!({"type": "input_text", "text": content}))
								}
								// -- Binary
								ContentPart::Binary(mut binary) => {
									let is_image = binary.is_image();

									// Process the image
									if is_image {
										let image_url = binary.into_url();
										let input_image = json!({
											"type": "input_image",
											"detail": "auto",
											"image_url": image_url
										});
										values.push(input_image);
									}
									// Process file
									// TODO - Needs to support audio
									else {
										let mut input_file = Map::new();
										input_file.insert("type".into(), "input_file".into());

										// Set the file name if not defined (otherwise error)
										if let Some(file_name) = binary.name.take() {
											input_file.insert("filename".into(), file_name.into());
										} else {
											unamed_file_count += 1;
											input_file
												.insert("filename".into(), format!("file-{unamed_file_count}").into());
										}

										let file_url = binary.into_url();
										if file_url.starts_with("data") {
											input_file.insert("file_data".into(), file_url.into());
										} else {
											input_file.insert("file_url".into(), file_url.into());
										}
										let input_file: Value = input_file.into();

										values.push(input_file);
									}
								}

								// Use `match` instead of `if let`. This will allow to future-proof this
								// implementation in case some new message content types would appear,
								// this way library would not compile if not all methods are implemented
								// continue would allow to gracefully skip pushing unserializable message
								// TODO: Probably need to warn if it is a ToolCalls type of content
								ContentPart::ToolCall(_) => (),
								ContentPart::ToolResponse(tool_response) => {
									// Extracted as an output item before this user message item
									// (see `embedded_output_items` above). Function outputs carry
									// their images natively in the output array; images rescued
									// from custom outputs are folded into this same user message
									// as `input_image` items, mirroring the Gemini serializer's
									// user-embedded handling.
									let mut rescued_images: Vec<Value> = Vec::new();
									embedded_output_items.push(tool_response_to_output_item(
										tool_response,
										&custom_call_ids,
										&mut rescued_images,
									));
									values.extend(rescued_images);
								}
								ContentPart::ThoughtSignature(_) => (),
								ContentPart::ReasoningContent(_) => (),
								// Custom are ignored for this logic
								ContentPart::Custom(_) => {}
							}
						}
						let had_embedded_tool_responses = !embedded_output_items.is_empty();
						if had_embedded_tool_responses {
							// Insert before the tool-image flush item emitted for this
							// iteration (if any), so the extracted output items stay adjacent
							// to the preceding tool-message run.
							let insert_at = flushed_images_at.unwrap_or(input_items.len());
							input_items.splice(insert_at..insert_at, embedded_output_items);
						}
						if values.is_empty() && had_embedded_tool_responses {
							// The user message carried only embedded tool responses; nothing is
							// left for it to say, so the now-empty user message item is omitted.
						} else {
							if cache_controlled {
								apply_resp_cache_breakpoint(model_iden, &mut values, "message")?;
							}
							input_items.push(json! ({"role": "user", "content": values}));
						}
					}
				}

				// Assistant - For now support Text and ToolCalls
				ChatRole::Assistant => {
					// Here we make sure if multiple text content part, we keep them in the same assistant message
					// In the new OpenAI Responses API, the tool call are just items out of assistant message
					let mut item_message_content: Vec<Value> = Vec::new();

					// Pre-pass: encrypted reasoning blobs from prior turns must be
					// carried back as top-level `{type: "reasoning"}` input items
					// to keep the Responses-API prefix cache warm. Without this,
					// even a verbatim resend of a prior turn re-processes every
					// token. They precede the assistant message they belong to,
					// mirroring the order the API emits them in the streaming
					// response. The blobs ride in on `ContentPart::ThoughtSignature`
					// parts (from `StreamEnd::captured_content`) or on
					// `ToolCall::thought_signatures` (rust-genai's streamer stashes
					// captured blobs there when there are tool calls).
					for part in msg.content.iter() {
						if let ContentPart::ThoughtSignature(blob) = part {
							input_items.push(json!({
								"type": "reasoning",
								"encrypted_content": blob,
								"summary": [],
							}));
						}
					}
					for part in msg.content.iter() {
						if let ContentPart::ToolCall(tool_call) = part
							&& let Some(sigs) = tool_call.thought_signatures.as_ref()
						{
							for blob in sigs {
								input_items.push(json!({
									"type": "reasoning",
									"encrypted_content": blob,
									"summary": [],
								}));
							}
						}
					}

					for part in msg.content {
						match part {
							ContentPart::Text(text) => {
								item_message_content.push(json!({
										"type": "output_text",
										"text": text
								}));
							}
							ContentPart::ToolCall(tool_call) => {
								// Make sure to create the assistant message
								if !item_message_content.is_empty() {
									input_items.push(json!({
										"type": "message",
										"role": "assistant",
										"content": item_message_content
									}));
									item_message_content = Vec::new();
								}
								if custom_tool_names.contains(&tool_call.fn_name) {
									let input = tool_call
										.fn_arguments
										.as_str()
										.map_or_else(|| tool_call.fn_arguments.to_string(), str::to_string);
									custom_call_ids.insert(tool_call.call_id.clone());
									input_items.push(json!({
										"type": "custom_tool_call",
										"call_id": tool_call.call_id,
										"name": tool_call.fn_name,
										"input": input,
									}));
								} else {
									// NOTE: Flatten for OpenAI Responses API.
									input_items.push(json!({
										"type": "function_call",
										"call_id": tool_call.call_id,
										"name": tool_call.fn_name,
										"arguments": tool_call.fn_arguments.to_string(),
									}));
								}
							}

							// TODO: Probably need towarn on this one (probably need to add binary here)
							ContentPart::Binary(_) => {}
							// No provider wire represents a tool result authored by the
							// assistant; fail loudly instead of dropping the content or
							// inventing a placement (use a Tool-role message).
							ContentPart::ToolResponse(_) => {
								return Err(assistant_embedded_tool_response_err(model_iden));
							}
							// ThoughtSignature and ReasoningContent are emitted as
							// top-level `type:reasoning` items in the pre-pass above.
							ContentPart::ThoughtSignature(_) => {}
							ContentPart::ReasoningContent(_) => {}
							// Custom are ignored for this logic
							ContentPart::Custom(_) => {}
						}
					}

					// Make sure we handle the rest of the assistant message
					if !item_message_content.is_empty() {
						input_items.push(json!({
							"type": "message",
							"role": "assistant",
							"content": item_message_content
						}));
					}
				}

				// Tool Response (Function tool call output)
				ChatRole::Tool => {
					for part in msg.content {
						if let ContentPart::ToolResponse(tool_response) = part {
							input_items.push(tool_response_to_output_item(
								tool_response,
								&custom_call_ids,
								&mut pending_custom_tool_images,
							));
						}
					}

					// TODO: Probably need to trace/warn that this will be ignored
				}
			}
		}

		// Flush custom-tool-result images from a trailing run of Tool messages.
		if !pending_custom_tool_images.is_empty() {
			input_items.push(tool_images_user_item(pending_custom_tool_images));
		}

		// -- Process the tools
		let tools = chat_req
			.tools
			.map(|tools| tools.into_iter().map(Self::tool_to_openai_tool).collect::<Result<Vec<Value>>>())
			.transpose()?;

		Ok(OpenAIRespRequestParts { input_items, tools })
	}

	fn tool_to_openai_tool(tool: Tool) -> Result<Value> {
		let Tool {
			name,
			description,
			schema,
			custom_format,
			strict,
			config,
			..
		} = tool;

		let name = match name {
			ToolName::WebSearch => "web_search".to_string(),
			ToolName::Custom(name) => name,
		};

		let tool_value = if let Some(format) = custom_format {
			json!({
				"type": "custom",
				"name": name,
				"description": description,
				"format": format,
			})
		} else {
			match name.as_ref() {
				"web_search" => {
					let mut tool_value = json!({"type": "web_search"});
					match config {
						Some(ToolConfig::WebSearch(_ws_config)) => {
							// FIXME: Implement what is posible in filters
						}
						Some(ToolConfig::Custom(config_value)) => {
							// IMPORTANT: Here like anthropic, we merge it on top of the toll value
							//            (and not as value of "name" as this would not fit that api)
							//            Gemini does a `{name: config}` which fit that API
							tool_value.x_merge(config_value)?;
						}
						None => (),
					};
					tool_value
				}
				name => {
					let strict = strict.unwrap_or(false);
					let parameters = tool_parameters_schema(schema, strict);

					json!({
						"type": "function",
						"name": name,
						"description": description,
						"parameters": parameters,
						"strict": strict,
					})
				}
			}
		};

		Ok(tool_value)
	}
}
// region:    --- Support

struct OpenAIRespRequestParts {
	input_items: Vec<Value>,
	tools: Option<Vec<Value>>,
}

/// Serialize a `ToolResponse` into a Responses API output item:
/// `custom_tool_call_output` when the `call_id` belongs to a custom tool call,
/// `function_call_output` otherwise (`call_id`s are not otherwise validated;
/// provider-side validation is the norm).
///
/// The Responses API natively supports `output` as an array of `input_text` /
/// `input_image` items for function call outputs, so their image parts ride in
/// the output array. Custom tool outputs are raw strings (with the
/// `tool_response_fallback_text` placeholder rules), so their images are
/// rescued into `rescued_custom_images`; the caller decides where they ride
/// (the batched follow-up user message item for Tool-role messages, or folded
/// into the carrying user message for user-embedded responses).
fn tool_response_to_output_item(
	tool_response: ToolResponse,
	custom_call_ids: &BTreeSet<String>,
	rescued_custom_images: &mut Vec<Value>,
) -> Value {
	let is_custom = custom_call_ids.contains(&tool_response.call_id);
	let response_type = if is_custom {
		"custom_tool_call_output"
	} else {
		"function_call_output"
	};
	let ToolResponse {
		call_id,
		content,
		parts,
		..
	} = tool_response;
	let parts = parts.unwrap_or_default();
	let has_parts = !parts.is_empty();

	let mut image_values: Vec<Value> = Vec::new();
	for binary in parts {
		if binary.is_image() {
			image_values.push(json!({
				"type": "input_image",
				"detail": "auto",
				"image_url": binary.into_url(),
			}));
		} else {
			warn!(
				"ToolResponse binary parts only support images for the OpenAI Responses adapter; skipping non-image part '{}'",
				binary.content_type
			);
		}
	}

	if is_custom {
		// NOTE: The fallback text applies only when parts were present, so
		//       plain text-only responses keep their exact legacy serialization.
		let output = if has_parts {
			tool_response_fallback_text(content, !image_values.is_empty())
		} else {
			content
		};
		rescued_custom_images.extend(image_values);
		json!({
			"type": response_type,
			"call_id": call_id,
			"output": output,
		})
	} else if image_values.is_empty() {
		json!({
			"type": response_type,
			"call_id": call_id,
			"output": content,
		})
	} else {
		let mut output: Vec<Value> = Vec::new();
		if !content.is_empty() {
			output.push(json!({"type": "input_text", "text": content}));
		}
		output.extend(image_values);
		json!({
			"type": response_type,
			"call_id": call_id,
			"output": output,
		})
	}
}

/// Build the follow-up `user` message input item that carries tool-result images
/// (`ToolResponse.parts`) rescued from custom tool outputs, since
/// `custom_tool_call_output.output` is a raw string and cannot include image blocks.
fn tool_images_user_item(image_values: Vec<Value>) -> Value {
	let mut content: Vec<Value> = Vec::with_capacity(image_values.len() + 1);
	content.push(json!({"type": "input_text", "text": TOOL_RESULT_IMAGES_LABEL}));
	content.extend(image_values);
	json!({"type": "message", "role": "user", "content": content})
}

fn apply_resp_cache_breakpoint(_model_iden: &ModelIden, content: &mut [Value], _scope: &'static str) -> Result<()> {
	let Some(content_block) = content.iter_mut().rev().find(|value| {
		matches!(
			value.get("type").and_then(Value::as_str),
			Some("input_text" | "input_image" | "input_file")
		)
	}) else {
		return Ok(());
	};

	content_block.x_insert("prompt_cache_breakpoint", json!({"mode": "explicit"}))?;
	Ok(())
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "adapter_impl_tests.rs"]
mod tests;

// endregion: --- Tests
