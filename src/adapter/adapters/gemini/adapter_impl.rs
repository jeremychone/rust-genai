use crate::adapter::adapters::support::get_api_key;
use crate::adapter::gemini::GeminiStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream, ChatStreamResponse,
	CompletionTokensDetails, ContentPart, ImageSource, MessageContent, ReasoningEffort, ToolCall, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebResponse, WebStream};
use crate::{Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Value, json};
use value_ext::JsonValueExt;

pub struct GeminiAdapter;

const MODELS: &[&str] = &[
	"gemini-2.0-flash",
	"gemini-2.0-flash-lite",
	"gemini-2.5-pro-preview-05-06",
	"gemini-1.5-pro",
];

// Per gemini doc (https://x.com/jeremychone/status/1916501987371438372)
const REASONING_ZERO: u32 = 0;
const REASONING_LOW: u32 = 1000;
const REASONING_MEDIUM: u32 = 8000;
const REASONING_HIGH: u32 = 24000;

// curl \
//   -H 'Content-Type: application/json' \
//   -d '{"contents":[{"parts":[{"text":"Explain how AI works"}]}]}' \
//   -X POST 'https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'

impl GeminiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GEMINI_API_KEY";
}

impl Adapter for GeminiAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	/// Note: For now, this returns the common models (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	/// NOTE: As Google Gemini has decided to put their API_KEY in the URL,
	///       this will return the URL without the API_KEY in it. The API_KEY will need to be added by the caller.
	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		let base_url = endpoint.base_url();
		let model_name = model.model_name.clone();
		match service_type {
			ServiceType::Chat => format!("{base_url}models/{model_name}:generateContent"),
			ServiceType::ChatStream => format!("{base_url}models/{model_name}:streamGenerateContent"),
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

		// -- Reasoning Budget
		let (model, reasoning_effort) = match (model, options_set.reasoning_effort()) {
			// No explicity reasoning_effor, try to infer from model name suffix (supports -zero)
			(model, None) => {
				let model_name: &str = &model.model_name;
				if let Some((prefix, last)) = model_name.rsplit_once('-') {
					let reasoning = match last {
						"zero" => Some(ReasoningEffort::Budget(REASONING_ZERO)),
						"low" => Some(ReasoningEffort::Budget(REASONING_LOW)),
						"medium" => Some(ReasoningEffort::Budget(REASONING_MEDIUM)),
						"high" => Some(ReasoningEffort::Budget(REASONING_HIGH)),
						_ => None,
					};
					// create the model name if there was a `-..` reasoning suffix
					let model = if reasoning.is_some() {
						model.with_name_or_clone(Some(prefix.to_string()))
					} else {
						model
					};

					(model, reasoning)
				} else {
					(model, None)
				}
			}
			// If reasoning effort, turn the low, medium, budget ones into Budget
			(model, Some(effort)) => {
				let effort = match effort {
					ReasoningEffort::Low => ReasoningEffort::Budget(REASONING_LOW),
					ReasoningEffort::Medium => ReasoningEffort::Budget(REASONING_MEDIUM),
					ReasoningEffort::High => ReasoningEffort::Budget(REASONING_HIGH),
					ReasoningEffort::Budget(budget) => ReasoningEffort::Budget(*budget),
				};
				(model, Some(effort))
			}
		};

		// -- parts
		let GeminiChatRequestParts {
			system,
			contents,
			tools,
		} = Self::into_gemini_request_parts(model.clone(), chat_req)?;

		// -- Playload
		let mut payload = json!({
			"contents": contents,
		});

		// -- Set the reasoning effort
		if let Some(ReasoningEffort::Budget(budget)) = reasoning_effort {
			payload.x_insert("/generationConfig/thinkingConfig/thinkingBudget", budget)?;
		}

		// -- headers (empty for gemini, since API_KEY is in url)
		let headers = vec![];

		// Note: It's unclear from the spec if the content of systemInstruction should have a role.
		//       Right now, it is omitted (since the spec states it can only be "user" or "model")
		//       It seems to work. https://ai.google.dev/api/rest/v1beta/models/generateContent
		if let Some(system) = system {
			payload.x_insert(
				"systemInstruction",
				json!({
					"parts": [ { "text": system }]
				}),
			)?;
		}

		// -- Tools
		if let Some(tools) = tools {
			payload.x_insert(
				"tools",
				json!({
					"function_declarations": tools
				}),
			)?;
		}

		// -- Response Format
		if let Some(ChatResponseFormat::JsonSpec(st_json)) = options_set.response_format() {
			// x_insert
			//     responseMimeType: "application/json",
			// responseSchema: {
			payload.x_insert("/generationConfig/responseMimeType", "application/json")?;
			let mut schema = st_json.schema.clone();
			schema.x_walk(|parent_map, name| {
				if name == "additionalProperties" {
					parent_map.remove("additionalProperties");
				}
				true
			});
			payload.x_insert("/generationConfig/responseSchema", schema)?;
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

		// -- url
		// NOTE: Somehow, Google decided to put the API key in the URL.
		//       This should be considered an antipattern from a security point of view
		//       even if it is done by the well respected Google. Everybody can make mistake once in a while.
		// e.g., '...models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'
		let url = Self::get_service_url(&model, service_type, endpoint);
		let url = format!("{url}?key={api_key}");

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
		let provider_model_iden = model_iden.with_name_or_clone(provider_model_name);

		let gemini_response = Self::body_to_gemini_chat_response(&model_iden.clone(), body)?;
		let GeminiChatResponse { content, usage } = gemini_response;

		let content = match content {
			Some(GeminiChatContent::Text(content)) => Some(MessageContent::from_text(content)),
			Some(GeminiChatContent::ToolCall(tool_call)) => Some(MessageContent::from_tool_calls(vec![tool_call])),
			None => None,
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
		let web_stream = WebStream::new_with_pretty_json_array(reqwest_builder);

		let gemini_stream = GeminiStreamer::new(web_stream, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(gemini_stream);

		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}
}

// region:    --- Support

/// Support functions for GeminiAdapter
impl GeminiAdapter {
	pub(super) fn body_to_gemini_chat_response(model_iden: &ModelIden, mut body: Value) -> Result<GeminiChatResponse> {
		// If the body has an `error` property, then it is assumed to be an error.
		if body.get("error").is_some() {
			return Err(Error::StreamEventError {
				model_iden: model_iden.clone(),
				body,
			});
		}

		let mut response = body.x_take::<Value>("/candidates/0/content/parts/0")?;
		let content = match response.x_take::<Value>("functionCall") {
			Ok(f) => Some(GeminiChatContent::ToolCall(ToolCall {
				call_id: f.x_get("name").unwrap_or("".to_string()), // TODO: Handle this, gemini does not return the call_id
				fn_name: f.x_get("name").unwrap_or("".to_string()),
				fn_arguments: f.x_get("args").unwrap_or(Value::Null),
			})),
			Err(_) => response
				.x_take::<Value>("text")
				.ok()
				.and_then(|v| v.as_str().map(String::from))
				.map(GeminiChatContent::Text),
		};
		let usage = body.x_take::<Value>("usageMetadata").map(Self::into_usage).unwrap_or_default();

		Ok(GeminiChatResponse { content, usage })
	}

	/// See gemini doc: https://ai.google.dev/api/generate-content#UsageMetadata
	pub(super) fn into_usage(mut usage_value: Value) -> Usage {
		let prompt_tokens: Option<i32> = usage_value.x_take("promptTokenCount").ok();
		let completion_tokens: Option<i32> = usage_value.x_take("candidatesTokenCount").ok();
		let total_tokens: Option<i32> = usage_value.x_take("totalTokenCount").ok();

		// IMPORTANT: For Gemini, the `thoughts_token_count` (~reasoning_tokens) is not included
		//            in the root `candidatesTokenCount` (~completion_tokens).
		//            Therefore, some computation is needed to normalize it in the "OpenAI API Way,"
		//            meaning `completion_tokens` represents the total of completion tokens,
		//            and the details provide a breakdown of the specific components.

		let (completion_tokens, completion_tokens_details) =
			match (completion_tokens, usage_value.x_get_i64("thoughtsTokenCount").ok()) {
				(Some(c_tokens), Some(t_tokens)) => {
					let t_tokens = t_tokens as i32; // should be safe enough
					(
						Some(c_tokens + t_tokens),
						Some(CompletionTokensDetails {
							accepted_prediction_tokens: None,
							rejected_prediction_tokens: None,
							reasoning_tokens: Some(t_tokens),
							audio_tokens: None,
						}),
					)
				}
				(None, Some(t_tokens)) => {
					(
						None,
						Some(CompletionTokensDetails {
							accepted_prediction_tokens: None,
							rejected_prediction_tokens: None,
							reasoning_tokens: Some(t_tokens as i32), // should be safe enough
							audio_tokens: None,
						}),
					)
				}
				(c_tokens, None) => (c_tokens, None),
			};

		Usage {
			prompt_tokens,
			// for now, None for Gemini
			prompt_tokens_details: None,

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
	fn into_gemini_request_parts(model_iden: ModelIden, chat_req: ChatRequest) -> Result<GeminiChatRequestParts> {
		let mut contents: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build
		for msg in chat_req.messages {
			match msg.role {
				// For now, system goes as "user" (later, we might have adapter_config.system_to_user_impl)
				ChatRole::System => {
					let MessageContent::Text(content) = msg.content else {
						return Err(Error::MessageContentTypeNotSupported {
							model_iden,
							cause: "Only MessageContent::Text supported for this model (for now)",
						});
					};
					systems.push(content)
				}
				ChatRole::User => {
					let content = match msg.content {
						MessageContent::Text(content) => json!([{"text": content}]),
						MessageContent::Parts(parts) => {
							json!(
								parts
									.iter()
									.map(|part| match part {
										ContentPart::Text(text) => json!({"text": text.clone()}),
										ContentPart::Image { content_type, source } => {
											match source {
												ImageSource::Url(url) => json!({
													"file_data": {
														"mime_type": content_type,
														"file_uri": url
													}
												}),
												ImageSource::Base64(content) => json!({
													"inline_data": {
														"mime_type": content_type,
														"data": content
													}
												}),
											}
										}
									})
									.collect::<Vec<Value>>()
							)
						}
						MessageContent::ToolCalls(tool_calls) => {
							json!(
								tool_calls
									.into_iter()
									.map(|tool_call| {
										json!({
											"functionCall": {
												"name": tool_call.fn_name,
												"args": tool_call.fn_arguments,
											}
										})
									})
									.collect::<Vec<Value>>()
							)
						}
						MessageContent::ToolResponses(tool_responses) => {
							json!(
								tool_responses
									.into_iter()
									.map(|tool_response| {
										json!({
											"functionResponse": {
												"name": tool_response.call_id,
												"response": {
													"name": tool_response.call_id,
													"content": serde_json::from_str(&tool_response.content).unwrap_or(Value::Null),
												}
											}
										})
									})
									.collect::<Vec<Value>>()
							)
						}
					};

					contents.push(json!({"role": "user", "parts": content}));
				}
				ChatRole::Assistant => {
					match msg.content {
						MessageContent::Text(content) => {
							contents.push(json!({"role": "model", "parts": [{"text": content}]}))
						}
						MessageContent::ToolCalls(tool_calls) => contents.push(json!({
							"role": "model",
							"parts": tool_calls
								.into_iter()
								.map(|tool_call| {
									json!({
										"functionCall": {
											"name": tool_call.fn_name,
											"args": tool_call.fn_arguments,
										}
									})
								})
								.collect::<Vec<Value>>()
						})),
						_ => {
							return Err(Error::MessageContentTypeNotSupported {
								model_iden,
								cause: "Only MessageContent::Text and MessageContent::ToolCalls supported for this model (for now)",
							});
						}
					};
				}
				ChatRole::Tool => {
					let content = match msg.content {
						MessageContent::ToolCalls(tool_calls) => {
							json!(
								tool_calls
									.into_iter()
									.map(|tool_call| {
										json!({
											"functionCall": {
												"name": tool_call.fn_name,
												"args": tool_call.fn_arguments,
											}
										})
									})
									.collect::<Vec<Value>>()
							)
						}
						MessageContent::ToolResponses(tool_responses) => {
							json!(
								tool_responses
									.into_iter()
									.map(|tool_response| {
										json!({
											"functionResponse": {
												"name": tool_response.call_id,
												"response": {
													"name": tool_response.call_id,
													"content": serde_json::from_str(&tool_response.content).unwrap_or(Value::Null),
												}
											}
										})
									})
									.collect::<Vec<Value>>()
							)
						}
						_ => {
							return Err(Error::MessageContentTypeNotSupported {
								model_iden,
								cause: "ChatRole::Tool can only be MessageContent::ToolCall or MessageContent::ToolResponse",
							});
						}
					};

					contents.push(json!({"role": "user", "parts": content}));
				}
			}
		}

		let system = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		let tools = chat_req.tools.map(|tools| {
			tools
				.into_iter()
				.map(|tool| {
					// TODO: Need to handle the error correctly
					// TODO: Needs to have a custom serializer (tool should not have to match to a provider)
					// NOTE: Right now, low probability, so, we just return null if cannot convert to value.
					json!({
						"name": tool.name,
						"description": tool.description,
						"parameters": tool.schema,
					})
				})
				.collect::<Vec<Value>>()
		});

		Ok(GeminiChatRequestParts {
			system,
			contents,
			tools,
		})
	}
}

// struct Gemini

pub(super) struct GeminiChatResponse {
	pub content: Option<GeminiChatContent>,
	pub usage: Usage,
}

pub(super) enum GeminiChatContent {
	Text(String),
	ToolCall(ToolCall),
}

struct GeminiChatRequestParts {
	system: Option<String>,
	/// The chat history (user and assistant, except for the last user message which is a message)
	contents: Vec<Value>,

	/// The tools to use
	tools: Option<Vec<Value>>,
}

// endregion: --- Support
