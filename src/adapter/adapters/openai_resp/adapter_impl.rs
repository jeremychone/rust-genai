use crate::adapter::adapters::support::get_api_key;
use crate::adapter::openai::OpenAIStreamer;
use crate::adapter::openai_resp::resp_types::RespResponse;
use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream, ChatStreamResponse,
	ContentPart, MessageContent, ReasoningEffort, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Headers, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{Map, Value, json};
use value_ext::JsonValueExt;

pub struct OpenAIRespAdapter;

// Latest models
const MODELS: &[&str] = &[
	//
	"gpt-5-codex",
];

impl OpenAIRespAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OPENAI_API_KEY";
}

impl Adapter for OpenAIRespAdapter {
	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.openai.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	/// Note: Currently returns the common models (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		Self::util_get_service_url(model, service_type, endpoint)
	}

	/// OpenAI Doc: https://platform.openai.com/docs/api-reference/responses/create
	///
	/// ## Note related to OpenAI Responses API
	/// - `.store = false` - To maintain consistent behavior with other chat completions, store is set to false
	/// - `.instructions` For now we do not use the top ".instructions" (genai::ChatRequest.system),
	///   but just add this top system as a regular system message.
	/// - `.summary` right now, supporting "generate reasoning summary" is not supported
	///
	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = target;
		let (model_name, _) = model.model_name.as_model_name_and_namespace();
		let adapter_kind = model.adapter_kind;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = AdapterDispatcher::get_service_url(&model, service_type, endpoint);

		// -- headers
		let mut headers = Headers::from(("Authorization".to_string(), format!("Bearer {api_key}")));

		// -- extra headers
		if let Some(extra_headers) = chat_options.extra_headers() {
			headers.merge_with(extra_headers);
		}

		// -- for new v1/responses/ for now do not support stream
		let stream = matches!(service_type, ServiceType::ChatStream);
		if stream {
			return Err(Error::AdapterNotSupported {
				adapter_kind,
				feature: "stream".into(),
			});
		}

		// -- compute reasoning_effort and eventual trimmed model_name
		// For now, just for openai AdapterKind
		let (reasoning_effort, model_name): (Option<ReasoningEffort>, &str) =
			if matches!(adapter_kind, AdapterKind::OpenAI) {
				let (reasoning_effort, model_name) = chat_options
					.reasoning_effort()
					.cloned()
					.map(|v| (Some(v), model_name))
					.unwrap_or_else(|| ReasoningEffort::from_model_name(model_name));

				(reasoning_effort, model_name)
			} else {
				(None, model_name)
			};

		// -- Build the basic payload
		let OpenAIRespRequestParts {
			input_items: messages,
			tools,
		} = Self::into_openai_request_parts(&model, chat_req)?;

		// IMPORTANT: `store = false` - To maintain consistent behavior with other chat completions, store is set to false
		let mut payload = json!({
			"store": false,
			"model": model_name,
			"input": messages
		});

		// -- Set reasoning effort
		if let Some(reasoning_effort) = reasoning_effort
			&& let Some(keyword) = reasoning_effort.as_keyword()
		{
			// NOTE: For now, we do not set the "summary" property to generate the reasoning summary

			payload.x_insert("reasoning", json!({"effort": keyword}))?;
			// TODO: needs to find a way to add summary: auto, concise, detailed
		}

		// -- Tools
		if let Some(tools) = tools {
			payload.x_insert("/tools", tools)?;
		}

		// -- Compute response format
		let response_format = if let Some(response_format) = chat_options.response_format() {
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

					// Flatten for OpenAI Responses
					Some(json!({
						"type": "json_schema",
						"name": st_json.name.clone(),
						"strict": true,
						// TODO: add description
						"schema": schema,
					}))
				}
			}
		} else {
			None
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
		if stream & chat_options.capture_usage().unwrap_or(false) {
			payload.x_insert("stream_options", json!({"include_usage": true}))?;
		}

		if let Some(temperature) = chat_options.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		if !chat_options.stop_sequences().is_empty() {
			payload.x_insert("stop", chat_options.stop_sequences())?;
		}

		if let Some(max_tokens) = chat_options.max_tokens() {
			payload.x_insert("max_tokens", max_tokens)?;
		}
		if let Some(top_p) = chat_options.top_p() {
			payload.x_insert("top_p", top_p)?;
		}
		if let Some(seed) = chat_options.seed() {
			payload.x_insert("seed", seed)?;
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
			usage,
			captured_raw_body,
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
	) -> String {
		let base_url = default_endpoint.base_url();
		// Parse into URL and query-params
		let base_url = reqwest::Url::parse(base_url).unwrap();
		let original_query_params = base_url.query().to_owned();

		let suffix = match service_type {
			ServiceType::Chat | ServiceType::ChatStream => "responses",
			ServiceType::Embed => "embeddings", // TODO: Probably needs to say not supported
		};
		let mut full_url = base_url.join(suffix).unwrap();
		full_url.set_query(original_query_params);
		full_url.to_string()
	}

	/// Takes the genai ChatMessages and builds the OpenAIChatRequestParts
	/// - `genai::ChatRequest.system`, if present, is added as the first message with role 'system'.
	/// - All messages get added with the corresponding roles (tools are not supported for now)
	///
	fn into_openai_request_parts(_model_iden: &ModelIden, chat_req: ChatRequest) -> Result<OpenAIRespRequestParts> {
		let mut input_items: Vec<Value> = Vec::new();

		// -- Process the system
		if let Some(system_msg) = chat_req.system {
			input_items.push(json!({"role": "system", "content": system_msg}));
		}

		let mut unamed_file_count = 0;

		// -- Process the messages
		for msg in chat_req.messages {
			// Note: Will handle more types later
			match msg.role {
				// For now, system and tool messages go to the system
				ChatRole::System => {
					if let Some(content) = msg.content.into_joined_texts() {
						input_items.push(json!({"role": "system", "content": content}))
					}
					// TODO: Probably need to warn if it is a ToolCalls type of content
				}

				// User - For now support Text and Binary
				ChatRole::User => {
					// -- If we have only text, then, we jjust returned the joined_texts
					if msg.content.is_text_only() {
						// NOTE: for now, if no content, just return empty string (respect current logic)
						let content = json!(msg.content.joined_texts().unwrap_or_else(String::new));
						input_items.push(json! ({"role": "user", "content": content}));
					} else {
						let mut values: Vec<Value> = Vec::new();

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
								ContentPart::ToolResponse(_) => (),
							}
						}
						input_items.push(json! ({"role": "user", "content": values}));
					}
				}

				// Assistant - For now support Text and ToolCalls
				ChatRole::Assistant => {
					// Here we make sure if multiple text content part, we keep them in the same assistant message
					// In the new OpenAI Responses API, the tool call are just items out of assistant message
					let mut item_message_content: Vec<Value> = Vec::new();

					for part in msg.content {
						match part {
							ContentPart::Text(text) => {
								item_message_content.push(json!({
										"type": "input_text",
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
								// NOTE: Flatten for OpenAI Responsess API
								input_items.push(json!({
									"type": "function_call",
									"call_id": tool_call.call_id,
									"name": tool_call.fn_name,
									"arguments": tool_call.fn_arguments.to_string(),
								}))
							}

							// TODO: Probably need towarn on this one (probably need to add binary here)
							ContentPart::Binary(_) => (),
							ContentPart::ToolResponse(_) => (),
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
							input_items.push(json!({
								"type": "function_call_output",
								"call_id": tool_response.call_id,
								"output": tool_response.content,
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
					// NOTE: Right now, low probability, so, we just return null if cannot convert to value.
					json!({
						"type": "function",
						"name": tool.name,
						"description": tool.description,
						"parameters": tool.schema,
						// TODO: If we need to support `strict: true` we need to add additionalProperties: false into the schema
						//       above (like structured output)
						"strict": false,
					})
				})
				.collect::<Vec<Value>>()
		});

		Ok(OpenAIRespRequestParts { input_items, tools })
	}
}

// region:    --- Support

struct OpenAIRespRequestParts {
	input_items: Vec<Value>,
	tools: Option<Vec<Value>>,
}

// endregion: --- Support
