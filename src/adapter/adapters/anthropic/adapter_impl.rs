use crate::ModelIden;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::anthropic::AnthropicStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, ContentPart, ImageSource,
	MessageContent, PromptTokensDetails, ToolCall, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{Value, json};
use tracing::warn;
use value_ext::JsonValueExt;

pub struct AnthropicAdapter;

// NOTE: For Anthropic, the max_tokens must be specified.
//       To avoid surprises, the default value for genai is the maximum for a given model.
// Current logic:
// - if model contains `3-opus` or `3-haiku` 4x max token limit,
// - otherwise assume 8k model
//
// NOTE: Will need to add the thinking option: https://docs.anthropic.com/en/docs/build-with-claude/extended-thinking
const MAX_TOKENS_8K: u32 = 8192;
const MAX_TOKENS_4K: u32 = 4096;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
	"claude-opus-4-20250514",
	"claude-sonnet-4-20250514",
	"claude-3-7-sonnet-latest",
	"claude-3-5-haiku-latest",
	"claude-3-opus-20240229",
	"claude-3-haiku-20240307",
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

	fn get_service_url(_model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		let base_url = endpoint.base_url();
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}messages"),
		}
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
		let url = Self::get_service_url(&model, service_type, endpoint);

		// -- headers
		let headers = vec![
			// headers
			("x-api-key".to_string(), api_key),
			("anthropic-version".to_string(), ANTHROPIC_VERSION.to_string()),
		];

		let model_name = model.model_name.clone();

		// -- Parts
		let AnthropicRequestParts {
			system,
			messages,
			tools,
		} = Self::into_anthropic_request_parts(model, chat_req)?;

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

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		let max_tokens = options_set.max_tokens().unwrap_or_else(|| {
			if model_name.contains("3-opus") || model_name.contains("3-haiku") {
				MAX_TOKENS_4K
			} else {
				MAX_TOKENS_8K
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
		_chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the provider_model_iden
		// TODO: Need to be implemented (if available), for now, just clone model_iden
		let provider_model_name: Option<String> = body.x_remove("model").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);

		// -- Capture the usage
		let usage = body.x_take::<Value>("usage");
		let usage = usage.map(Self::into_usage).unwrap_or_default();

		// -- Capture the content
		// NOTE: Anthropic supports a list of content of multiple types but not the ChatResponse
		//       So, the strategy is to:
		//       - List all of the content and capture the text and tool_use
		//       - If there is one or more tool_use, this will take precedence and MessageContent will support tool_call list
		//       - Otherwise, the text is concatenated
		// NOTE: We need to see if the multiple content type text happens and why. If not, we can probably simplify this by just capturing the first one.
		//       Eventually, ChatResponse will have `content: Option<Vec<MessageContent>>` for the multi parts (with images and such)
		let content_items: Vec<Value> = body.x_take("content")?;

		let mut text_content: Vec<String> = Vec::new();
		// Note: here tool_calls is probably the exception, so not creating the vector if not needed
		let mut tool_calls: Option<Vec<ToolCall>> = None;

		for mut item in content_items {
			let typ: &str = item.x_get_as("type")?;
			if typ == "text" {
				text_content.push(item.x_take("text")?);
			} else if typ == "tool_use" {
				let call_id = item.x_take::<String>("id")?;
				let fn_name = item.x_take::<String>("name")?;
				// if not found, will be Value::Null
				let fn_arguments = item.x_take::<Value>("input").unwrap_or_default();
				let tool_call = ToolCall {
					call_id,
					fn_name,
					fn_arguments,
				};
				tool_calls.get_or_insert_with(Vec::new).push(tool_call);
			}
		}

		let content = if let Some(tool_calls) = tool_calls {
			Some(MessageContent::from(tool_calls))
		} else {
			Some(MessageContent::from(text_content.join("\n")))
		};

		Ok(ChatResponse {
			content,
			reasoning_content: None,
			model_iden,
			provider_model_iden,
			usage,
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
}

// region:    --- Support

impl AnthropicAdapter {
	pub(super) fn into_usage(mut usage_value: Value) -> Usage {
		let prompt_tokens: Option<i32> = usage_value.x_take("input_tokens").ok();
		let completion_tokens: Option<i32> = usage_value.x_take("output_tokens").ok();
		let cache_creation_input_tokens: i32 = usage_value.x_take("cache_creation_input_tokens").unwrap_or(0);
		let cache_read_input_tokens: i32 = usage_value.x_take("cache_read_input_tokens").unwrap_or(0);

		// Compute total_tokens
		let total_tokens = if prompt_tokens.is_some() || completion_tokens.is_some() {
			Some(
				prompt_tokens.unwrap_or(0)
					+ completion_tokens.unwrap_or(0)
					+ cache_creation_input_tokens
					+ cache_read_input_tokens,
			)
		} else {
			None
		};

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
			prompt_tokens,
			prompt_tokens_details,

			completion_tokens,
			// for now, None for Anthropic
			completion_tokens_details: None,

			total_tokens,
		}
	}

	/// Takes the GenAI ChatMessages and constructs the System string and JSON Messages for Anthropic.
	/// - Will push the `ChatRequest.system` and system message to `AnthropicRequestParts.system`
	fn into_anthropic_request_parts(_model_iden: ModelIden, chat_req: ChatRequest) -> Result<AnthropicRequestParts> {
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
				// for now, system and tool messages go to the system
				ChatRole::System => {
					if let MessageContent::Text(content) = msg.content {
						systems.push((content, is_cache_control))
					}
					// TODO: Needs to trace/warn that other types are not supported
				}
				ChatRole::User => {
					let content = match msg.content {
						MessageContent::Text(content) => apply_cache_control_to_text(is_cache_control, content),
						MessageContent::Parts(parts) => {
							let values = parts
								.iter()
								.filter_map(|part| match part {
									ContentPart::Text(text) => Some(json!({"type": "text", "text": text})),
									ContentPart::Image { content_type, source } => match source {
										ImageSource::Url(_) => {
											// TODO: Might need to return an error here.
											warn!(
												"Anthropic doesn't support images from URL, need to handle it gracefully"
											);
											None
										}
										ImageSource::Base64(content) => Some(json!({
											"type": "image",
											"source": {
												"type": "base64",
												"media_type": content_type,
												"data": content,
											},
										})),
									},
								})
								.collect::<Vec<Value>>();

							let values = apply_cache_control_to_parts(is_cache_control, values);

							json!(values)
						}
						// Use `match` instead of `if let`. This will allow to future-proof this
						// implementation in case some new message content types would appear,
						// this way the library would not compile if not all methods are implemented
						// continue would allow to gracefully skip pushing unserializable message
						// TODO: Probably need to warn if it is a ToolCalls type of content
						MessageContent::ToolCalls(_) => continue,
						MessageContent::ToolResponses(_) => continue,
					};
					messages.push(json! ({"role": "user", "content": content}));
				}
				ChatRole::Assistant => {
					//
					match msg.content {
						MessageContent::Text(content) => {
							let content = apply_cache_control_to_text(is_cache_control, content);
							messages.push(json! ({"role": "assistant", "content": content}))
						}
						MessageContent::ToolCalls(tool_calls) => {
							let tool_calls = tool_calls
								.into_iter()
								.map(|tool_call| {
									// see: https://docs.anthropic.com/en/docs/build-with-claude/tool-use#example-of-successful-tool-result
									json!({
										"type": "tool_use",
										"id": tool_call.call_id,
										"name": tool_call.fn_name,
										"input": tool_call.fn_arguments,
									})
								})
								.collect::<Vec<Value>>();
							let tool_calls = apply_cache_control_to_parts(is_cache_control, tool_calls);
							messages.push(json! ({
								"role": "assistant",
								"content": tool_calls
							}));
						}
						// TODO: Probably need to trace/warn that this will be ignored
						MessageContent::Parts(_) => (),
						MessageContent::ToolResponses(_) => (),
					}
				}
				ChatRole::Tool => {
					if let MessageContent::ToolResponses(tool_responses) = msg.content {
						let tool_responses = tool_responses
							.into_iter()
							.map(|tool_response| {
								json!({
									"type": "tool_result",
									"content": tool_response.content,
									"tool_use_id": tool_response.call_id,
								})
							})
							.collect::<Vec<Value>>();
						let tool_responses = apply_cache_control_to_parts(is_cache_control, tool_responses);
						// FIXME: MessageContent::ToolResponse should be MessageContent::ToolResponses (even if OpenAI does require multi Tool message)
						messages.push(json!({
							"role": "user",
							"content": tool_responses
						}));
					}
					// TODO: Probably need to trace/warn that this will be ignored
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
