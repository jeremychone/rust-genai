use crate::adapter::adapters::support::get_api_key;
use crate::adapter::openai::OpenAIStreamer;
use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream, ChatStreamResponse,
	ContentPart, ImageSource, MessageContent, MetaUsage, ToolCall,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde::Deserialize;
use serde_json::{json, Value};
use value_ext::JsonValueExt;

pub struct OpenAIAdapter;

// Latest models
const MODELS: &[&str] = &[
	//
	"gpt-4o",
	"gpt-4o-mini",
	"o1-preview",
	"o1-mini",
];

impl OpenAIAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OPENAI_API_KEY";
}

impl Adapter for OpenAIAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.openai.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	/// Note: Currently returns the common models (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		Self::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options)
	}

	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the usage
		let usage = body.x_take("usage").map(OpenAIAdapter::into_usage).unwrap_or_default();

		// -- Capture the content
		let content = if let Some(mut first_choice) = body.x_take::<Option<Value>>("/choices/0")? {
			if let Some(content) = first_choice
				.x_take::<Option<String>>("/message/content")?
				.map(MessageContent::from)
			{
				Some(content)
			} else {
				first_choice
					.x_take("/message/tool_calls")
					.ok()
					.map(parse_tool_calls)
					.transpose()?
					.map(MessageContent::from_tool_calls)
			}
		} else {
			None
		};

		Ok(ChatResponse {
			content,
			model_iden,
			usage,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_sets: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let openai_stream = OpenAIStreamer::new(event_source, model_iden.clone(), options_sets);
		let chat_stream = ChatStream::from_inter_stream(openai_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}
}

/// Support functions for other adapters that share OpenAI APIs
impl OpenAIAdapter {
	pub(in crate::adapter::adapters) fn util_get_service_url(
		_model: &ModelIden,
		service_type: ServiceType,
		// -- utility arguments
		default_endpoint: Endpoint,
	) -> String {
		let base_url = default_endpoint.base_url();
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}chat/completions"),
		}
	}

	pub(in crate::adapter::adapters) fn util_to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint);

		// -- headers
		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let stream = matches!(service_type, ServiceType::ChatStream);

		// -- Build the basic payload
		let model_name = model.model_name.to_string();
		let OpenAIRequestParts { messages, tools } = Self::into_openai_request_parts(model, chat_req)?;
		let mut payload = json!({
			"model": model_name,
			"messages": messages,
			"stream": stream
		});

		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}

		// -- Add options
		let response_format = if let Some(response_format) = options_set.response_format() {
			match response_format {
				ChatResponseFormat::JsonMode => Some(json!({"type": "json_object"})),
				ChatResponseFormat::JsonSpec(st_json) => {
					// "type": "json_schema", "json_schema": {...}

					let mut schema = st_json.schema.clone();
					schema.x_walk(|parent_map, name| {
						if name == "type" {
							let typ = parent_map.get("type").and_then(|v| v.as_str()).unwrap_or("");
							if typ == "object" {
								parent_map.insert("additionalProperties".to_string(), false.into());
							}
						}
						true
					});

					Some(json!({
						"type": "json_schema",
						"json_schema": {
							"name": st_json.name.clone(),
							"strict": true,
							// TODO: add description
							"schema": schema,
						}
					}))
				}
			}
		} else {
			None
		};

		if let Some(response_format) = response_format {
			payload["response_format"] = response_format;
		}

		// -- Add supported ChatOptions
		if stream & options_set.capture_usage().unwrap_or(false) {
			payload.x_insert("stream_options", json!({"include_usage": true}))?;
		}

		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !options_set.stop_sequences().is_empty() {
			payload.x_insert("stop", options_set.stop_sequences())?;
		}

		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("max_tokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	/// Note: Needs to be called from super::streamer as well
	pub(super) fn into_usage(mut usage_value: Value) -> MetaUsage {
		let input_tokens: Option<i32> = usage_value.x_take("prompt_tokens").ok();
		let output_tokens: Option<i32> = usage_value.x_take("completion_tokens").ok();
		let total_tokens: Option<i32> = usage_value.x_take("total_tokens").ok();
		MetaUsage {
			input_tokens,
			output_tokens,
			total_tokens,
		}
	}

	/// Takes the genai ChatMessages and builds the OpenAIChatRequestParts
	/// - `genai::ChatRequest.system`, if present, is added as the first message with role 'system'.
	/// - All messages get added with the corresponding roles (tools are not supported for now)
	fn into_openai_request_parts(_model_iden: ModelIden, chat_req: ChatRequest) -> Result<OpenAIRequestParts> {
		let mut messages: Vec<Value> = Vec::new();

		// -- Process the system
		if let Some(system_msg) = chat_req.system {
			messages.push(json!({"role": "system", "content": system_msg}));
		}

		// -- Process the messages
		for msg in chat_req.messages {
			// Note: Will handle more types later
			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => {
					if let MessageContent::Text(content) = msg.content {
						messages.push(json!({"role": "system", "content": content}))
					}
					// TODO: Probably need to warn if it is a ToolCalls type of content
				}
				ChatRole::User => {
					let content = match msg.content {
						MessageContent::Text(content) => json!(content),
						MessageContent::Parts(parts) => {
							json!(parts
								.iter()
								.map(|part| match part {
									ContentPart::Text(text) => json!({"type": "text", "text": text.clone()}),
									ContentPart::Image { content_type, source } => {
										match source {
											ImageSource::Url(url) => {
												json!({"type": "image_url", "image_url": {"url": url}})
											}
											ImageSource::Base64(content) => {
												let image_url = format!("data:{content_type};base64,{content}");
												json!({"type": "image_url", "image_url": {"url": image_url}})
											}
										}
									}
								})
								.collect::<Vec<Value>>())
						}
						// Use `match` instead of `if let`. This will allow to future-proof this
						// implementation in case some new message content types would appear,
						// this way library would not compile if not all methods are implemented
						// continue would allow to gracefully skip pushing unserializable message
						// TODO: Probably need to warn if it is a ToolCalls type of content
						MessageContent::ToolCalls(_) => continue,
						MessageContent::ToolResponses(_) => continue,
					};
					messages.push(json! ({"role": "user", "content": content}));
				}

				ChatRole::Assistant => match msg.content {
					MessageContent::Text(content) => messages.push(json! ({"role": "assistant", "content": content})),
					MessageContent::ToolCalls(tool_calls) => {
						let tool_calls = tool_calls
							.into_iter()
							.map(|tool_call| {
								json!({
									"type": "function",
									"id": tool_call.call_id,
									"function": {
										"name": tool_call.fn_name,
										"arguments": tool_call.fn_arguments.to_string(),
									}
								})
							})
							.collect::<Vec<Value>>();
						messages.push(json! ({"role": "assistant", "tool_calls": tool_calls}))
					}
					// TODO: Probably need to trace/warn that this will be ignored
					MessageContent::Parts(_) => (),
					MessageContent::ToolResponses(_) => (),
				},

				ChatRole::Tool => {
					if let MessageContent::ToolResponses(tool_responses) = msg.content {
						for tool_response in tool_responses {
							messages.push(json!({
								"role": "tool",
								"content": tool_response.content,
								"tool_call_id": tool_response.call_id,
							}))
						}
					}
					// TODO: Probably need to trace/warn that this will be ignored
				}
			}
		}

		// -- Process the tools
		let tools = chat_req.tools.map(|tools| {
			tools
				.into_iter()
				.map(|tool| {
					// TODO: Need to handle the error correctly
					// TODO: Needs to have a custom serializer (tool should not have to match to a provider)
					// NOTE: Right now, low probability, so, we just return null if cannto to value.
					json!({
						"type": "function",
						"function": {
							"name": tool.name,
							"description": tool.description,
							"parameters": tool.schema,
							// TODO: If we need to support `strict: true` we need to add additionalProperties: false into the schema
							//       above (like structured output)
							"strict": false,
						}
					})
				})
				.collect::<Vec<Value>>()
		});

		Ok(OpenAIRequestParts { messages, tools })
	}
}

// region:    --- Support

struct OpenAIRequestParts {
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

fn parse_tool_calls(raw_tool_calls: Value) -> Result<Vec<ToolCall>> {
	let Value::Array(raw_tool_calls) = raw_tool_calls else {
		return Err(Error::InvalidJsonResponseElement {
			info: "tool calls is not an array",
		});
	};

	let tool_calls = raw_tool_calls.into_iter().map(parse_tool_call).collect::<Result<Vec<_>>>()?;

	Ok(tool_calls)
}

fn parse_tool_call(raw_tool_call: Value) -> Result<ToolCall> {
	// Define a helper struct to match the original JSON structure.
	#[derive(Deserialize)]
	struct IterimToolFnCall {
		id: String,
		#[allow(unused)]
		#[serde(rename = "type")]
		r#type: String,
		function: IterimFunction,
	}

	#[derive(Deserialize)]
	struct IterimFunction {
		name: String,
		arguments: Value,
	}

	let iterim = serde_json::from_value::<IterimToolFnCall>(raw_tool_call)?;

	let fn_name = iterim.function.name;

	// For now support Object only, and parse the eventual string as a json value.
	// Eventually, we might check pricing
	let fn_arguments = match iterim.function.arguments {
		Value::Object(obj) => Value::Object(obj),
		Value::String(txt) => serde_json::from_str(&txt)?,
		_ => {
			return Err(Error::InvalidJsonResponseElement {
				info: "tool call arguments is not an object",
			})
		}
	};

	// Then, map the fields of the helper struct to the flat structure.
	Ok(ToolCall {
		call_id: iterim.id,
		fn_name,
		fn_arguments,
	})
}

// endregion: --- Support
