use crate::adapter::anthropic::AnthropicStreamer;
use crate::adapter::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, MessageContent, MetaUsage,
	ToolCall,
};
use crate::webc::WebResponse;
use crate::Result;
use crate::{ClientConfig, ModelIden};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};
use value_ext::JsonValueExt;

pub struct AnthropicAdapter;

const BASE_URL: &str = "https://api.anthropic.com/v1/";

// NOTE: For Anthropic, the max_tokens must be specified.
//       To avoid surprises, the default value for genai is the maximum for a given model.
// The 3-5 models have an 8k max token limit, while the 3 models have a 4k limit.
const MAX_TOKENS_8K: u32 = 8192;
const MAX_TOKENS_4K: u32 = 4096;

const ANTRHOPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
	"claude-3-5-sonnet-20241022",
	"claude-3-5-haiku-20241022",
	"claude-3-opus-20240229",
	"claude-3-haiku-20240307",
];

impl Adapter for AnthropicAdapter {
	fn default_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("ANTHROPIC_API_KEY")
	}

	/// Note: For now, it returns the common models (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(_model_iden: ModelIden, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}messages"),
		}
	}

	fn to_web_request_data(
		model_iden: ModelIden,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let model_name = model_iden.model_name.clone();

		let stream = matches!(service_type, ServiceType::ChatStream);
		let url = Self::get_service_url(model_iden.clone(), service_type);

		// -- api_key (this Adapter requires it)
		let api_key = get_api_key(model_iden.clone(), client_config)?;

		let headers = vec![
			// headers
			("x-api-key".to_string(), api_key.to_string()),
			("anthropic-version".to_string(), ANTRHOPIC_VERSION.to_string()),
		];

		let AnthropicRequestParts {
			system,
			messages,
			tools,
		} = Self::into_anthropic_request_parts(model_iden.clone(), chat_req)?;

		// -- Build the basic payload
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

		if options_set.stop_sequences().len() > 0 {
			payload.x_insert("stop_sequences", options_set.stop_sequences())?;
		}

		let max_tokens = options_set.max_tokens().unwrap_or_else(|| {
			if model_iden.model_name.contains("3-5") {
				MAX_TOKENS_8K
			} else {
				MAX_TOKENS_4K
			}
		});
		payload.x_insert("max_tokens", max_tokens)?; // required for Anthropic

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the usage
		let usage = body.x_take("usage").map(Self::into_usage).unwrap_or_default();

		// -- Capture the content
		// NOTE: Anthropic support a list of content of multitypes but not the ChatResponse
		//       So, the strategy is to:
		//       - List all of the content and capture the text and tool_use
		//       - If there is one or more tool_use, this will take precedence and MessageContent support tool_call list
		//       - Otherwise, the text is concatenated
		// NOTE: We need to see if the multiple content type text happens and why. If not, we can probably simplify this by just capturing the first one.
		//       Eventually, ChatResponse will have `content: Option<Vec<MessageContent>>` for the multi parts (with images and such)
		let content_items: Vec<Value> = body.x_take("content")?;

		let mut text_content: Vec<String> = Vec::new();
		// Note: here tool_calls is probably the exception, so, not creating the vector if not needed
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
			model_iden,
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
	pub(super) fn into_usage(mut usage_value: Value) -> MetaUsage {
		let input_tokens: Option<i32> = usage_value.x_take("input_tokens").ok();
		let output_tokens: Option<i32> = usage_value.x_take("output_tokens").ok();

		// Compute total_tokens
		let total_tokens = if input_tokens.is_some() || output_tokens.is_some() {
			Some(input_tokens.unwrap_or(0) + output_tokens.unwrap_or(0))
		} else {
			None
		};

		MetaUsage {
			input_tokens,
			output_tokens,
			total_tokens,
		}
	}

	/// Takes the GenAI ChatMessages and constructs the System string and JSON Messages for Anthropic.
	/// - Will push the `ChatRequest.system` and system message to `AnthropicRequestParts.system`
	fn into_anthropic_request_parts(_model_iden: ModelIden, chat_req: ChatRequest) -> Result<AnthropicRequestParts> {
		let mut messages: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Process the messages
		for msg in chat_req.messages {
			match msg.role {
				// for now, system and tool messages go to system
				ChatRole::System => {
					if let MessageContent::Text(content) = msg.content {
						systems.push(content)
					}
					// TODO: Needs to trace/warn that other type are not supported
				}
				ChatRole::User => {
					if let MessageContent::Text(content) = msg.content {
						messages.push(json! ({"role": "user", "content": content}))
					}
					// TODO: Needs to trace/warn that other type are not supported
				}
				ChatRole::Assistant => {
					//
					match msg.content {
						MessageContent::Text(content) => {
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
							messages.push(json! ({
								"role": "assistant",
								"content": tool_calls
							}));
						}
						// TODO: Probably need to trace/warn that this will be ignored
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

						// FIXME: MessageContent::ToolResponse should be MessageContent::ToolResponses (even if openAI does require multi Tool message)
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
			Some(systems.join("\n"))
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
					// NOTE: Right now, low probability, so, we just return null if cannto to value.
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

struct AnthropicRequestParts {
	system: Option<String>,
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

// endregion: --- Support
