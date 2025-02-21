use crate::adapter::adapters::support::get_api_key;
use crate::adapter::gemini::GeminiStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatResponseFormat, ChatRole, ChatStream, ChatStreamResponse,
	ContentPart, ImageSource, MessageContent, MetaUsage, ToolCall,
};
use crate::embed::EmbedResponse;
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebResponse, WebStream};
use crate::{embed, Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
use value_ext::JsonValueExt;

pub struct GeminiAdapter;

const MODELS: &[&str] = &[
	"gemini-2.0-flash",
	"gemini-1.5-pro",
	"gemini-1.5-flash",
	"gemini-1.5-flash-8b",
	"gemini-1.0-pro",
	"gemini-1.5-flash-latest",
];

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

	fn get_embed_url(model: &ModelIden, endpoint: Endpoint) -> Option<String> {
		let base_url = endpoint.base_url();
		let model_name = model.model_name.clone();

		Some(format!("{base_url}models/{model_name}:embedContent"))
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
		// NOTE: Somehow, Google decided to put the API key in the URL.
		//       This should be considered an antipattern from a security point of view
		//       even if it is done by the well respected Google. Everybody can make mistake once in a while.
		// e.g., '...models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'
		let url = Self::get_service_url(&model, service_type, endpoint);
		let url = format!("{url}?key={api_key}");

		// -- parts
		let GeminiChatRequestParts {
			system,
			contents,
			tools,
		} = Self::into_gemini_request_parts(model, chat_req)?;

		// -- Playload
		let mut payload = json!({
			"contents": contents,
		});

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

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { body, .. } = web_response;

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

	fn embed(
		service_target: ServiceTarget,
		embed_req: crate::embed::SingleEmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = service_target;
		let model_name = &model.model_name;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_embed_url(&model, endpoint).ok_or(Error::EmbeddingNotSupported {
			model_iden: model.clone(),
		})?;
		let url = format!("{url}?key={api_key}");

		// -- headers (empty for gemini, since API_KEY is in url)
		let headers = vec![];

		let mut payload = json!({
			"model": format!("models/{model_name}"),
			"content": {
				"parts": [{
					"text": embed_req.document
				}]
			},
		});

		if let Some(dimensions) = options_set.dimensions() {
			payload.x_insert("outputDimensionality", dimensions)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn embed_batch(
		service_target: ServiceTarget,
		embed_req: crate::embed::BatchEmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = service_target;
		let model_name = &model.model_name;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_embed_url(&model, endpoint)
			.ok_or(Error::EmbeddingNotSupported {
				model_iden: model.clone(),
			})?
			// todo: this might not be the best way to do this
			.replace("embedContent", "batchEmbedContents");
		let url = format!("{url}?key={api_key}");

		// -- headers (empty for gemini, since API_KEY is in url)
		let headers = vec![];

		let payload = json!({
			"requests": embed_req
				.documents
				.into_iter()
				.filter_map(|document| {
					let mut request = json!({
						"model": format!("models/{model_name}"),
						"content": {
							"parts": [{
								"text": document
							}]
						}
					});

					if let Some(dimensions) = options_set.dimensions() {
						request.x_insert("outputDimensionality", dimensions).ok()?;
					}

					Some(request)
				})
				.collect::<Vec<Value>>()
		});

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		let WebResponse { mut body, .. } = web_response;

		// -- Capture the usage (Gemini does not return usage)
		let usage = embed::MetaUsage::default();

		// -- Capture the content, assume single embedding first
		let single_embedding = body
			.x_take("embedding")
			.map(|mut embedding: Value| {
				let embedding = embedding.x_take("values").ok()?;

				// Gemini does not return the index
				Some(embed::EmbeddingObject { index: None, embedding })
			})
			.ok()
			.flatten();

		if let Some(single_embedding) = single_embedding {
			let embeddings = vec![single_embedding];

			return Ok(EmbedResponse {
				embeddings,
				usage,
				model_iden,
			});
		}

		// -- Capture the content, assume batch embedding or error if all goes wrong
		let embeddings = body.x_take("embeddings").map(|embeddings: Vec<Value>| {
			embeddings
				.into_iter()
				.filter_map(|mut embedding: Value| {
					let embedding = embedding.x_take("values").ok()?;

					// Gemini does not return the index
					Some(embed::EmbeddingObject { index: None, embedding })
				})
				.collect::<Vec<embed::EmbeddingObject>>()
		})?;

		Ok(EmbedResponse {
			embeddings,
			usage,
			model_iden,
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

	pub(super) fn into_usage(mut usage_value: Value) -> MetaUsage {
		let prompt_tokens: Option<i32> = usage_value.x_take("promptTokenCount").ok();
		let completion_tokens: Option<i32> = usage_value.x_take("candidatesTokenCount").ok();
		let total_tokens: Option<i32> = usage_value.x_take("totalTokenCount").ok();

		// legacy
		let input_tokens = prompt_tokens;
		let output_tokens = prompt_tokens;

		#[allow(deprecated)]
		MetaUsage {
			prompt_tokens,
			// for now, None for Gemini
			prompt_tokens_details: None,

			completion_tokens,
			// for now, None for Gemini
			completion_tokens_details: None,

			total_tokens,

			// -- Legacy
			input_tokens,
			output_tokens,
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
							json!(parts
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
								.collect::<Vec<Value>>())
						}
						MessageContent::ToolCalls(tool_calls) => {
							json!(tool_calls
								.into_iter()
								.map(|tool_call| {
									json!({
										"functionCall": {
											"name": tool_call.fn_name,
											"args": tool_call.fn_arguments,
										}
									})
								})
								.collect::<Vec<Value>>())
						}
						MessageContent::ToolResponses(tool_responses) => {
							json!(tool_responses
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
								.collect::<Vec<Value>>())
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
							json!(tool_calls
								.into_iter()
								.map(|tool_call| {
									json!({
										"functionCall": {
											"name": tool_call.fn_name,
											"args": tool_call.fn_arguments,
										}
									})
								})
								.collect::<Vec<Value>>())
						}
						MessageContent::ToolResponses(tool_responses) => {
							json!(tool_responses
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
								.collect::<Vec<Value>>())
						}
						_ => {
							return Err(Error::MessageContentTypeNotSupported {
								model_iden,
								cause: "ChatRole::Tool can only be MessageContent::ToolCall or MessageContent::ToolResponse"
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
	pub usage: MetaUsage,
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
