use crate::adapter::adapters::support::get_api_key;
use crate::adapter::anthropic::AnthropicStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse,
	ContentPart, MessageContent, PromptTokensDetails, ReasoningEffort, ToolCall, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Headers, ModelIden};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{Value, json};
use tracing::warn;
use value_ext::JsonValueExt;

pub struct AnthropicAdapter;

const REASONING_LOW: u32 = 1024;
const REASONING_MEDIUM: u32 = 8000;
const REASONING_HIGH: u32 = 24000;

// NOTE: For Anthropic, the max_tokens must be specified.
//       To avoid surprises, the default value for genai is the maximum for a given model.
// Current logic:
// - if model contains `3-opus` or `3-haiku` 4x max token limit,
// - otherwise assume 8k model
//
// NOTE: Will need to add the thinking option: https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking
// For max model tokens see: https://docs.anthropic.com/en/docs/about-claude/models/overview
//
// fall back
const MAX_TOKENS_64K: u32 = 64000; // claude-3-7-sonnet, claude-sonnet-4.x, claude-haiku-4-5
// custom
const MAX_TOKENS_32K: u32 = 32000; // claude-opus-4
const MAX_TOKENS_8K: u32 = 8192; // claude-3-5-sonnet, claude-3-5-haiku
const MAX_TOKENS_4K: u32 = 4096; // claude-3-opus, claude-3-haiku

const ANTHROPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
	"claude-opus-4-1-20250805",
	"claude-sonnet-4-5-20250929",
	"claude-haiku-4-5-20251001",
];

impl AnthropicAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "ANTHROPIC_API_KEY";
}

impl Adapter for AnthropicAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.anthropic.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	/// Note: For now, it returns the common models (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		let url = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}messages"),
			ServiceType::Embed => format!("{base_url}embeddings"), // Anthropic doesn't support embeddings yet
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

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// -- headers
		let headers = Headers::from(vec![
			// headers
			("x-api-key".to_string(), api_key),
			("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
		]);

		// -- Parts
		let AnthropicRequestParts {
			system,
			messages,
			tools,
		} = Self::into_anthropic_request_parts(chat_req)?;

		// -- Extract Model Name and Reasoning
		let (raw_model_name, _) = model.model_name.as_model_name_and_namespace();

		let (model_name, thinking_budget) = match (raw_model_name, options_set.reasoning_effort()) {
			// No explicity reasoning_effor, try to infer from model name suffix (supports -zero)
			(model, None) => {
				// let model_name: &str = &model.model_name;
				if let Some((prefix, last)) = raw_model_name.rsplit_once('-') {
					let reasoning = match last {
						"zero" => None,    // That will disable thinking
						"minimal" => None, // That will disable thinking
						"low" => Some(REASONING_LOW),
						"medium" => Some(REASONING_MEDIUM),
						"high" => Some(REASONING_HIGH),
						_ => None,
					};
					// create the model name if there was a `-..` reasoning suffix
					let model = if reasoning.is_some() { prefix } else { model };
					(model, reasoning)
				} else {
					(model, None)
				}
			}
			// If reasoning effort, turn the low, medium, budget ones into Budget
			(model, Some(effort)) => {
				let effort = match effort {
					// -- When minimal, same a zeror
					ReasoningEffort::Minimal => None,
					ReasoningEffort::Low => Some(REASONING_LOW),
					ReasoningEffort::Medium => Some(REASONING_MEDIUM),
					ReasoningEffort::High => Some(REASONING_HIGH),
					ReasoningEffort::Budget(budget) => Some(*budget),
				};
				(model, effort)
			}
		};

		// -- Build the basic payload
		let stream = matches!(service_type, ServiceType::ChatStream);
		let mut payload = json!({
			"model": model_name.to_string(),
			"messages": messages,
			"stream": stream
		});

		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}

		// -- Set the reasoning effort
		if let Some(budget) = thinking_budget {
			payload.x_insert(
				"thinking",
				json!({
					"type": "enabled",
					"budget_tokens": budget
				}),
			)?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		//const MAX_TOKENS_64K: u32 = 64000; // claude-sonnet-4, claude-3-7-sonnet,
		// custom
		// const MAX_TOKENS_32K: u32 = 32000; // claude-opus-4
		// const MAX_TOKENS_8K: u32 = 8192; // claude-3-5-sonnet, claude-3-5-haiku
		// const MAX_TOKENS_4K: u32 = 4096; // claude-3-opus, claude-3-haiku
		let max_tokens = options_set.max_tokens().unwrap_or_else(|| {
			// most likely models used, so put first. Also a little wider with `claude-sonnet` (since name from version 4)
			if model_name.contains("claude-sonnet")
				|| model_name.contains("claude-haiku")
				|| model_name.contains("claude-3-7-sonnet")
			{
				MAX_TOKENS_64K
			} else if model_name.contains("claude-opus-4") {
				MAX_TOKENS_32K
			} else if model_name.contains("claude-3-5") {
				MAX_TOKENS_8K
			} else if model_name.contains("3-opus") || model_name.contains("3-haiku") {
				MAX_TOKENS_4K
			}
			// for now, fall back on the 64K by default (might want to be more conservative)
			else {
				MAX_TOKENS_64K
			}
		});
		payload.x_insert("max_tokens", max_tokens)?; // required for Anthropic

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		let captured_raw_body = options_set.capture_raw_body().unwrap_or_default().then(|| body.clone());

		// -- Capture the provider_model_iden
		// TODO: Need to be implemented (if available), for now, just clone model_iden
		let provider_model_name: Option<String> = body.x_remove("model").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);

		// -- Capture the usage
		let usage = body.x_take::<Value>("usage");
		let usage = usage.map(Self::into_usage).unwrap_or_default();

		// -- Capture the content
		let mut content: MessageContent = MessageContent::default();

		// NOTE: Here we are going to concatenate all of the Anthropic text content items into one
		//       genai MessageContent::Text. This is more in line with the OpenAI API style,
		//       but loses the fact that they were originally separate items.
		let json_content_items: Vec<Value> = body.x_take("content")?;

		let mut reasoning_content: Vec<String> = Vec::new();

		for mut item in json_content_items {
			let typ: &str = item.x_get_as("type")?;
			match typ {
				"text" => {
					let part = ContentPart::from_text(item.x_take::<String>("text")?);
					content.push(part);
				}
				"thinking" => reasoning_content.push(item.x_take("thinking")?),
				"tool_use" => {
					let call_id = item.x_take::<String>("id")?;
					let fn_name = item.x_take::<String>("name")?;
					// if not found, will be Value::Null
					let fn_arguments = item.x_take::<Value>("input").unwrap_or_default();
					let tool_call = ToolCall {
						call_id,
						fn_name,
						fn_arguments,
						thought_signatures: None,
					};

					let part = ContentPart::ToolCall(tool_call);
					content.push(part);
				}
				_ => (),
			}
		}

		let reasoning_content = if !reasoning_content.is_empty() {
			Some(reasoning_content.join("\n"))
		} else {
			None
		};

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden,
			provider_model_iden,
			usage,
			captured_raw_body,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let anthropic_stream = AnthropicStreamer::new(event_source, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(anthropic_stream);
		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		_service_target: crate::ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::Anthropic,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: crate::ModelIden,
		_web_response: crate::webc::WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::Anthropic,
			feature: "embeddings".to_string(),
		})
	}
}

// region:    --- Support

impl AnthropicAdapter {
	pub(super) fn into_usage(mut usage_value: Value) -> Usage {
		// IMPORTANT: For Anthropic, the `input_tokens` does not include `cache_creation_input_tokens` or `cache_read_input_tokens`.
		// Therefore, it must be normalized in the OpenAI style, where it includes both cached and written tokens (for symmetry).
		let input_tokens: i32 = usage_value.x_take("input_tokens").ok().unwrap_or(0);
		let cache_creation_input_tokens: i32 = usage_value.x_take("cache_creation_input_tokens").unwrap_or(0);
		let cache_read_input_tokens: i32 = usage_value.x_take("cache_read_input_tokens").unwrap_or(0);
		let completion_tokens: i32 = usage_value.x_take("output_tokens").ok().unwrap_or(0);

		// compute the prompt_tokens
		let prompt_tokens = input_tokens + cache_creation_input_tokens + cache_read_input_tokens;

		// Compute total_tokens
		let total_tokens = prompt_tokens + completion_tokens;

		// For now the logic is to have a Some of PromptTokensDetails if at least one of those value is not 0
		// TODO: Needs to be normalized across adapters.
		let prompt_tokens_details = if cache_creation_input_tokens > 0 || cache_read_input_tokens > 0 {
			Some(PromptTokensDetails {
				cache_creation_tokens: Some(cache_creation_input_tokens),
				cached_tokens: Some(cache_read_input_tokens),
				audio_tokens: None,
			})
		} else {
			None
		};

		Usage {
			prompt_tokens: Some(prompt_tokens),
			prompt_tokens_details,

			completion_tokens: Some(completion_tokens),
			// for now, None for Anthropic
			completion_tokens_details: None,

			total_tokens: Some(total_tokens),
		}
	}

	/// Takes the GenAI ChatMessages and constructs the System string and JSON Messages for Anthropic.
	/// - Will push the `ChatRequest.system` and system message to `AnthropicRequestParts.system`
	fn into_anthropic_request_parts(chat_req: ChatRequest) -> Result<AnthropicRequestParts> {
		let mut messages: Vec<Value> = Vec::new();
		// (content, is_cache_control)
		let mut systems: Vec<(String, bool)> = Vec::new();

		// NOTE: For now, this means the first System cannot have a cache control
		//       so that we do not change too much.
		if let Some(system) = chat_req.system {
			systems.push((system, false));
		}

		// -- Process the messages
		for msg in chat_req.messages {
			let is_cache_control = msg.options.map(|o| o.cache_control.is_some()).unwrap_or(false);

			match msg.role {
				// Collect only text for system; other content parts are ignored by Anthropic here.
				ChatRole::System => {
					if let Some(system_text) = msg.content.joined_texts() {
						systems.push((system_text, is_cache_control));
					}
				}

				// User message: text, binary (image/document), and tool_result supported.
				ChatRole::User => {
					if msg.content.is_text_only() {
						let text = msg.content.joined_texts().unwrap_or_else(String::new);
						let content = apply_cache_control_to_text(is_cache_control, text);
						messages.push(json!({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();
						for part in msg.content {
							match part {
								ContentPart::Text(text) => {
									values.push(json!({"type": "text", "text": text}));
								}
								ContentPart::Binary(binary) => {
									let is_image = binary.is_image();
									let Binary {
										content_type, source, ..
									} = binary;

									if is_image {
										match &source {
											BinarySource::Url(_) => {
												// As of this API version, Anthropic doesn't support images by URL directly in messages.
												warn!(
													"Anthropic doesn't support images from URL, need to handle it gracefully"
												);
											}
											BinarySource::Base64(content) => {
												values.push(json!({
													"type": "image",
													"source": {
														"type": "base64",
														"media_type": content_type,
														"data": content,
													}
												}));
											}
										}
									} else {
										match &source {
											BinarySource::Url(url) => {
												values.push(json!({
													"type": "document",
													"source": {
														"type": "url",
														"url": url,
													}
												}));
											}
											BinarySource::Base64(b64) => {
												values.push(json!({
													"type": "document",
													"source": {
														"type": "base64",
														"media_type": content_type,
														"data": b64,
													}
												}));
											}
										}
									}
								}
								// ToolCall is not valid in user content for Anthropic; skip gracefully.
								ContentPart::ToolCall(_tc) => {}
								ContentPart::ToolResponse(tool_response) => {
									values.push(json!({
										"type": "tool_result",
										"content": tool_response.content,
										"tool_use_id": tool_response.call_id,
									}));
								}
								ContentPart::ThoughtSignature(_) => {}
							}
						}
						let values = apply_cache_control_to_parts(is_cache_control, values);
						messages.push(json!({"role": "user", "content": values}));
					}
				}

				// Assistant can mix text and tool_use entries.
				ChatRole::Assistant => {
					let mut values: Vec<Value> = Vec::new();
					let mut has_tool_use = false;
					let mut has_text = false;

					for part in msg.content {
						match part {
							ContentPart::Text(text) => {
								has_text = true;
								values.push(json!({"type": "text", "text": text}));
							}
							ContentPart::ToolCall(tool_call) => {
								has_tool_use = true;
								// see: https://docs.anthropic.com/en/docs/build-with-claude/tool-use#example-of-successful-tool-result
								values.push(json!({
									"type": "tool_use",
									"id": tool_call.call_id,
									"name": tool_call.fn_name,
									"input": tool_call.fn_arguments,
								}));
							}
							// Unsupported for assistant role in Anthropic message content
							ContentPart::Binary(_) => {}
							ContentPart::ToolResponse(_) => {}
							ContentPart::ThoughtSignature(_) => {}
						}
					}

					if !has_tool_use && has_text && !is_cache_control && values.len() == 1 {
						// Optimize to simple string when it's only one text part and no cache control.
						let text = values
							.first()
							.and_then(|v| v.get("text"))
							.and_then(|v| v.as_str())
							.unwrap_or_default()
							.to_string();
						let content = apply_cache_control_to_text(false, text);
						messages.push(json!({"role": "assistant", "content": content}));
					} else {
						let values = apply_cache_control_to_parts(is_cache_control, values);
						messages.push(json!({"role": "assistant", "content": values}));
					}
				}

				// Tool responses are represented as user tool_result items in Anthropic.
				ChatRole::Tool => {
					let mut values: Vec<Value> = Vec::new();
					for part in msg.content {
						if let ContentPart::ToolResponse(tool_response) = part {
							values.push(json!({
								"type": "tool_result",
								"content": tool_response.content,
								"tool_use_id": tool_response.call_id,
							}));
						}
					}
					if !values.is_empty() {
						let values = apply_cache_control_to_parts(is_cache_control, values);
						messages.push(json!({"role": "user", "content": values}));
					}
				}
			}
		}

		// -- Create the Anthropic system
		// NOTE: Anthropic does not have a "role": "system", just a single optional system property
		let system = if !systems.is_empty() {
			let mut last_cache_idx = -1;
			// first determine the last cache control index
			for (idx, (_, is_cache_control)) in systems.iter().enumerate() {
				if *is_cache_control {
					last_cache_idx = idx as i32;
				}
			}
			// Now build the system multi part
			let system: Value = if last_cache_idx > 0 {
				let mut parts: Vec<Value> = Vec::new();
				for (idx, (content, _)) in systems.iter().enumerate() {
					let idx = idx as i32;
					if idx == last_cache_idx {
						let part = json!({"type": "text", "text": content, "cache_control": {"type": "ephemeral"}});
						parts.push(part);
					} else {
						let part = json!({"type": "text", "text": content});
						parts.push(part);
					}
				}
				json!(parts)
			} else {
				let content_buff = systems.iter().map(|(content, _)| content.as_str()).collect::<Vec<&str>>();
				// we add empty line in between each system
				let content = content_buff.join("\n\n");
				json!(content)
			};
			Some(system)
		} else {
			None
		};

		// -- Process the tools
		let tools = chat_req.tools.map(|tools| {
			tools
				.into_iter()
				.map(|tool| {
					// TODO: Need to handle the error correctly
					// TODO: Needs to have a custom serializer (tool should not have to match to a provider)
					// NOTE: Right now, low probability, so we just return null if cannot convert to value.
					let mut tool_value = json!({
						"name": tool.name,
						"input_schema": tool.schema,
					});

					if let Some(description) = tool.description {
						// TODO: need to handle error
						let _ = tool_value.x_insert("description", description);
					}
					tool_value
				})
				.collect::<Vec<Value>>()
		});

		Ok(AnthropicRequestParts {
			system,
			messages,
			tools,
		})
	}
}

/// Apply the cache control logic to a text content
fn apply_cache_control_to_text(is_cache_control: bool, content: String) -> Value {
	if is_cache_control {
		let value = json!({"type": "text", "text": content, "cache_control": {"type": "ephemeral"}});
		json!(vec![value])
	}
	// simple return
	else {
		json!(content)
	}
}

/// Apply the cache control logic to a text content
fn apply_cache_control_to_parts(is_cache_control: bool, parts: Vec<Value>) -> Vec<Value> {
	let mut parts = parts;
	if is_cache_control && !parts.is_empty() {
		let len = parts.len();
		if let Some(last_value) = parts.get_mut(len - 1) {
			// NOTE: For now, if it fails, then, no cache
			let _ = last_value.x_insert("cache_control", json!( {"type": "ephemeral"}));
			// TODO: Should warn
		}
	}
	parts
}

struct AnthropicRequestParts {
	system: Option<Value>,
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

// endregion: --- Support
