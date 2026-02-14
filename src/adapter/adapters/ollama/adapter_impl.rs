//! API DOC: <https://github.com/ollama/ollama/blob/main/docs/api.md>

use crate::Headers;
use crate::adapter::ollama::OllamaStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse, ContentPart,
	MessageContent, Tool, ToolCall, ToolName, Usage,
};
use crate::embed::{EmbedResponse, Embedding};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{Value, json};
use value_ext::JsonValueExt;

pub struct OllamaAdapter;

// region:    --- Adapter Impl

impl Adapter for OllamaAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = None;

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "http://localhost:11434/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::from_single("ollama"),
		}
	}

	async fn all_model_names(adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let endpoint = Self::default_endpoint();
		let base_url = endpoint.base_url();
		let url = format!("{base_url}api/tags");

		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &[]).await.map_err(|webc_error| Error::WebAdapterCall {
			adapter_kind,
			webc_error,
		})?;

		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("name")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}

	fn get_service_url(_model_iden: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		let base_url = endpoint.base_url();
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => Ok(format!("{base_url}api/chat")),
			ServiceType::Embed => Ok(format!("{base_url}api/embed")),
		}
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, endpoint, .. } = target;

		// -- Service URL
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// -- Ollama Request Parts
		let OllamaRequestParts { messages, tools } = Self::into_ollama_request_parts(chat_req)?;

		// -- Ollama Options
		let mut options = json!({});
		if let Some(temperature) = options_set.temperature() {
			options.x_insert("temperature", temperature)?;
		}
		if let Some(top_p) = options_set.top_p() {
			options.x_insert("top_p", top_p)?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			options.x_insert("num_predict", max_tokens)?;
		}
		if let Some(seed) = options_set.seed() {
			options.x_insert("seed", seed)?;
		}
		if !options_set.stop_sequences().is_empty() {
			options.x_insert("stop", options_set.stop_sequences())?;
		}

		// -- Build Payload
		let stream = matches!(service_type, ServiceType::ChatStream);
		let (_, model_name) = model.model_name.namespace_and_name();

		let mut payload = json!({
			"model": model_name,
			"messages": messages,
			"stream": stream,
		});

		if !options.as_object().unwrap().is_empty() {
			payload.x_insert("options", options)?;
		}

		if let Some(tools) = tools {
			payload.x_insert("tools", tools)?;
		}

		if let Some(format) = options_set.response_format() {
			// Note: Ollama's API uses "format": "json" for its JSON mode, so we set that if the chat options specify json mode.
			if matches!(format, crate::chat::ChatResponseFormat::JsonMode) {
				payload.x_insert("format", "json")?;
			}
		}

		// -- Headers
		let mut headers = Headers::default();
		if let Some(extra_headers) = options_set.extra_headers() {
			headers.merge_with(extra_headers);
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		let captured_raw_body = if chat_options.capture_raw_body().unwrap_or(false) {
			Some(body.clone())
		} else {
			None
		};

		// -- Content and Tool Calls
		let mut message: Value = body.x_take("message")?;
		let content_text: Option<String> = message.x_take("content").ok();
		let mut content = content_text.map(MessageContent::from_text).unwrap_or_default();

		// -- Reasoning Content
		// Ollama API doc mentions `thinking` field in message object.
		// Some models (like DeepSeek) might also use `reasoning_content`.
		let reasoning_content: Option<String> = message
			.x_take::<String>("thinking")
			.or_else(|_| message.x_take::<String>("reasoning_content"))
			.ok();

		if let Ok(tcs_value) = message.x_take::<Vec<Value>>("tool_calls") {
			for mut tc_val in tcs_value {
				let fn_name: String = tc_val.x_take("/function/name")?;
				let fn_arguments: Value = tc_val.x_take("/function/arguments")?;

				// Generate a call_id if missing (genai requires one)
				let call_id = tc_val
					.x_take::<String>("/id")
					.unwrap_or_else(|_| format!("call_{}", &uuid::Uuid::new_v4().to_string()[..8]));

				content.push(ToolCall {
					call_id,
					fn_name,
					fn_arguments,
					thought_signatures: None,
				});
			}
		}

		// -- Usage
		let usage = Self::into_usage(&mut body);

		Ok(ChatResponse {
			content,
			reasoning_content,
			model_iden: model_iden.clone(),
			provider_model_iden: model_iden,
			usage,
			captured_raw_body,
		})
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let streamer = OllamaStreamer::new(
			crate::webc::WebStream::new_with_delimiter(reqwest_builder, "\n"),
			model_iden.clone(),
			options_set,
		);
		Ok(ChatStreamResponse {
			stream: ChatStream::from_inter_stream(streamer),
			model_iden,
		})
	}

	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		let ServiceTarget { model, endpoint, .. } = service_target;
		let url = Self::get_service_url(&model, ServiceType::Embed, endpoint)?;

		let (_, model_name) = model.model_name.namespace_and_name();

		let mut payload = json!({
			"model": model_name,
			"input": embed_req.inputs(),
		});

		if let Some(dimensions) = options_set.dimensions() {
			payload.x_insert("dimensions", dimensions)?;
		}
		if let Some(truncate) = options_set.truncate() {
			payload.x_insert("truncate", truncate)?;
		}

		// -- Headers
		let mut headers = Headers::default();
		if let Some(extra_headers) = options_set.headers() {
			headers.merge_with(extra_headers);
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		let WebResponse { mut body, .. } = web_response;

		let captured_raw_body = if options_set.capture_raw_body() {
			Some(body.clone())
		} else {
			None
		};

		let embeddings_raw: Vec<Vec<f32>> = body.x_take("embeddings")?;
		let embeddings = embeddings_raw
			.into_iter()
			.enumerate()
			.map(|(index, vector)| Embedding::new(vector, index))
			.collect();

		let usage = Self::into_usage(&mut body);

		Ok(EmbedResponse {
			embeddings,
			model_iden: model_iden.clone(),
			provider_model_iden: model_iden,
			usage,
			captured_raw_body,
		})
	}
}

// endregion: --- Adapter Impl

// region:    --- Support

impl OllamaAdapter {
	fn into_usage(body: &mut Value) -> Usage {
		let prompt_tokens = body.x_take::<i32>("prompt_eval_count").ok();
		let completion_tokens = body.x_take::<i32>("eval_count").ok();
		let total_tokens = match (prompt_tokens, completion_tokens) {
			(Some(p), Some(c)) => Some(p + c),
			_ => None,
		};

		Usage {
			prompt_tokens,
			completion_tokens,
			total_tokens,
			..Default::default()
		}
	}

	/// Takes the GenAI ChatMessages and constructs the JSON Messages for Ollama.
	fn into_ollama_request_parts(chat_req: ChatRequest) -> Result<OllamaRequestParts> {
		let mut messages = Vec::new();

		// -- System
		if let Some(system) = chat_req.system {
			messages.push(json!({
				"role": "system",
				"content": system,
			}));
		}

		// -- Messages
		for msg in chat_req.messages {
			let mut ollama_msg = json!({
				"role": msg.role.to_string().to_lowercase(),
			});

			let mut content = String::new();
			let mut images = Vec::new();
			let mut tool_calls = Vec::new();

			for part in msg.content {
				match part {
					ContentPart::Text(txt) => content.push_str(&txt),
					ContentPart::Binary(Binary {
						content_type, source, ..
					}) => {
						if content_type.starts_with("image/") {
							// Note: Ollama native API expects images in base64 format in a field named "images" as an array.
							if let BinarySource::Base64(data) = source {
								images.push(data);
							}
						}
					}
					ContentPart::ToolCall(tool_call) => {
						tool_calls.push(json!({
							"function": {
								"name": tool_call.fn_name,
								"arguments": tool_call.fn_arguments,
							}
						}));
					}
					ContentPart::ToolResponse(tr) => {
						// Note: Ollama native API expects role "tool" for tool response
						ollama_msg.x_insert("content", tr.content)?;
					}
					_ => {}
				}
			}

			if !content.is_empty() {
				ollama_msg.x_insert("content", content)?;
			}
			if !images.is_empty() {
				ollama_msg.x_insert("images", images)?;
			}
			if !tool_calls.is_empty() {
				ollama_msg.x_insert("tool_calls", tool_calls)?;
			}

			messages.push(ollama_msg);
		}

		// -- Tools
		let tools = chat_req
			.tools
			.map(|tools| tools.into_iter().map(Self::tool_to_ollama_tool).collect::<Result<Vec<Value>>>())
			.transpose()?;

		Ok(OllamaRequestParts { messages, tools })
	}

	fn tool_to_ollama_tool(tool: Tool) -> Result<Value> {
		let Tool {
			name,
			description,
			schema,
			..
		} = tool;

		let name = match name {
			ToolName::WebSearch => "web_search".to_string(),
			ToolName::Custom(name) => name,
		};

		let mut tool_value = json!({
			"type": "function",
			"function": {
				"name": name,
			}
		});

		if let Some(description) = description {
			tool_value.x_insert("/function/description", description)?;
		}
		if let Some(parameters) = schema {
			tool_value.x_insert("/function/parameters", parameters)?;
		}

		Ok(tool_value)
	}
}

struct OllamaRequestParts {
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

// endregion: --- Support
