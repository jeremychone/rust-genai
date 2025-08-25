use crate::adapter::adapters::support::get_api_key;
use crate::adapter::gemini::GeminiStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream,
	ChatStreamResponse, CompletionTokensDetails, ContentPart, MessageContent, PromptTokensDetails, ReasoningEffort,
	ToolCall, Usage,
};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebResponse, WebStream};
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Value, json};
use value_ext::JsonValueExt;

pub struct GeminiAdapter;

// Note: Those model names are just informative, as the Gemini AdapterKind is selected on `startsWith("gemini")`
const MODELS: &[&str] = &[
	//
	"gemini-2.5-pro",
	"gemini-2.5-flash",
	"gemini-2.5-flash-lite",
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
		let (model_name, _) = model.model_name.as_model_name_and_namespace();
		match service_type {
			ServiceType::Chat => format!("{base_url}models/{model_name}:generateContent"),
			ServiceType::ChatStream => format!("{base_url}models/{model_name}:streamGenerateContent"),
			ServiceType::Embed => format!("{base_url}models/{model_name}:embedContent"), // Gemini embeddings API
		}
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;
		let (model_name, _) = model.model_name.as_model_name_and_namespace();

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- headers (empty for gemini)
		let headers = Headers::from(("x-goog-api-key".to_string(), api_key.to_string()));

		// -- Reasoning Budget
		let (provider_model_name, reasoning_effort) = match (model_name, options_set.reasoning_effort()) {
			// No explicity reasoning_effor, try to infer from model name suffix (supports -zero)
			(model, None) => {
				// let model_name: &str = &model.model_name;
				if let Some((prefix, last)) = model_name.rsplit_once('-') {
					let reasoning = match last {
						"zero" => Some(ReasoningEffort::Budget(REASONING_ZERO)),
						"low" => Some(ReasoningEffort::Budget(REASONING_LOW)),
						"medium" => Some(ReasoningEffort::Budget(REASONING_MEDIUM)),
						"high" => Some(ReasoningEffort::Budget(REASONING_HIGH)),
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
					// -- for now, match minimal to Low (because zero is not supported by 2.5 pro)
					ReasoningEffort::Minimal => ReasoningEffort::Budget(REASONING_LOW),
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
		} = Self::into_gemini_request_parts(&model, chat_req)?;

		// -- Playload
		let mut payload = json!({
			"contents": contents,
		});

		// -- Set the reasoning effort
		if let Some(ReasoningEffort::Budget(budget)) = reasoning_effort {
			payload.x_insert("/generationConfig/thinkingConfig/thinkingBudget", budget)?;
		}

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
			payload.x_insert("tools", tools)?;
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
		let provider_model = model.from_name(provider_model_name);
		let url = Self::get_service_url(&provider_model, service_type, endpoint);
		let url = url.to_string();

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
		let provider_model_name: Option<String> = body.x_remove("modelVersion").ok();
		let provider_model_iden = model_iden.from_optional_name(provider_model_name);
		let gemini_response = Self::body_to_gemini_chat_response(&model_iden.clone(), body)?;
		let GeminiChatResponse {
			content: gemini_content,
			usage,
		} = gemini_response;

		// FIXME: Needs to take the content list
		let mut tool_calls: Vec<ToolCall> = Default::default();
		let mut content: Vec<MessageContent> = Default::default();
		// let mut text_content:
		for g_item in gemini_content {
			match g_item {
				GeminiChatContent::Text(text) => content.push(MessageContent::from_text(text)),
				GeminiChatContent::ToolCall(tool_call) => tool_calls.push(tool_call),
			}
		}
		if !tool_calls.is_empty() {
			content.push(MessageContent::ToolCalls(tool_calls))
		}

		Ok(ChatResponse {
			content,
			reasoning_content: None,
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
		let web_stream = WebStream::new_with_pretty_json_array(reqwest_builder);

		let gemini_stream = GeminiStreamer::new(web_stream, model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(gemini_stream);

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
	pub(super) fn body_to_gemini_chat_response(model_iden: &ModelIden, mut body: Value) -> Result<GeminiChatResponse> {
		// If the body has an `error` property, then it is assumed to be an error.
		if body.get("error").is_some() {
			return Err(Error::StreamEventError {
				model_iden: model_iden.clone(),
				body,
			});
		}

		let mut content: Vec<GeminiChatContent> = Vec::new();

		// -- Read multipart
		let parts = body.x_take::<Vec<Value>>("/candidates/0/content/parts")?;
		for mut part in parts {
			// -- Capture eventual function call
			if let Ok(fn_call_value) = part.x_take::<Value>("functionCall") {
				let tool_call = ToolCall {
					// NOTE: Gemini does not have call_id so, use name
					call_id: fn_call_value.x_get("name").unwrap_or("".to_string()), // TODO: Handle this, gemini does not return the call_id
					fn_name: fn_call_value.x_get("name").unwrap_or("".to_string()),
					fn_arguments: fn_call_value.x_get("args").unwrap_or(Value::Null),
				};
				content.push(GeminiChatContent::ToolCall(tool_call))
			}

			// -- Capture eventual text
			if let Some(txt_content) = part
				.x_take::<Value>("text")
				.ok()
				.and_then(|v| if let Value::String(v) = v { Some(v) } else { None })
				.map(GeminiChatContent::Text)
			{
				content.push(txt_content)
			}
		}
		let usage = body.x_take::<Value>("usageMetadata").map(Self::into_usage).unwrap_or_default();

		Ok(GeminiChatResponse { content, usage })
	}

	/// See gemini doc: https://ai.google.dev/api/generate-content#UsageMetadata
	pub(super) fn into_usage(mut usage_value: Value) -> Usage {
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
		let prompt_tokens_details = g_cached_tokens.map(|g_cached_tokens| PromptTokensDetails {
			cache_creation_tokens: None,
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
	fn into_gemini_request_parts(
		model_iden: &ModelIden, // use for error reporting
		chat_req: ChatRequest,
	) -> Result<GeminiChatRequestParts> {
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
							model_iden: model_iden.clone(),
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
										ContentPart::Binary {
											name: _name,
											content_type,
											source,
										} => {
											match source {
												BinarySource::Url(url) => json!({
													"file_data": {
														"mime_type": content_type,
														"file_uri": url
													}
												}),
												BinarySource::Base64(content) => json!({
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
													"content": tool_response.content,
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
								model_iden: model_iden.clone(),
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
													"content": tool_response.content,
												}
											}
										})
									})
									.collect::<Vec<Value>>()
							)
						}
						_ => {
							return Err(Error::MessageContentTypeNotSupported {
								model_iden: model_iden.clone(),
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

		// -- Build tools
		let tools = if let Some(req_tools) = chat_req.tools {
			let mut tools: Vec<Value> = Vec::new();
			// Note: This is to add only one function_declarations in the tools as per the gemini spec
			//       The rest are builtins
			let mut function_declarations: Vec<Value> = Vec::new();
			for req_tool in req_tools {
				// -- if it is a builtin tool
				if matches!(
					req_tool.name.as_str(),
					"googleSearch" | "googleSearchRetrieval" | "codeExecution" | "urlContext"
				) {
					tools.push(json!({req_tool.name: req_tool.config}));
				}
				// -- otherwise, user tool
				else {
					function_declarations.push(json! {
						{
							"name": req_tool.name,
							"description": req_tool.description,
							"parameters": req_tool.schema,
						}
					})
				}
			}
			if !function_declarations.is_empty() {
				tools.push(json!({"function_declarations": function_declarations}));
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
}

// struct Gemini

/// FIXME: need to be Vec<GeminiChatContent>
pub(super) struct GeminiChatResponse {
	pub content: Vec<GeminiChatContent>,
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
