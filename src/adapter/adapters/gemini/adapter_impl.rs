use crate::adapter::gemini::GeminiStreamer;
use crate::adapter::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, MessageContent, MetaUsage,
};
use crate::support::value_ext::ValueExt;
use crate::webc::{WebResponse, WebStream};
use crate::{ClientConfig, ModelInfo};
use crate::{Error, Result};
use reqwest::RequestBuilder;
use serde_json::{json, Value};

pub struct GeminiAdapter;

const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/";
const MODELS: &[&str] = &[
	"gemini-1.5-pro",
	"gemini-1.5-flash",
	"gemini-1.0-pro",
	"gemini-1.5-flash-latest",
];

// curl \
//   -H 'Content-Type: application/json' \
//   -d '{"contents":[{"parts":[{"text":"Explain how AI works"}]}]}' \
//   -X POST 'https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'

impl Adapter for GeminiAdapter {
	fn default_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("GEMINI_API_KEY")
	}

	/// Note: For now returns the common ones (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(_model_info: ModelInfo, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => BASE_URL.to_string(),
		}
	}

	fn to_web_request_data(
		model_info: ModelInfo,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let api_key = get_api_key(model_info.clone(), client_config)?;

		// For gemini, the service url returned is just the base url
		// since model and API key is part of the url (see below)
		let url = Self::get_service_url(model_info.clone(), service_type);

		// e.g., '...models/gemini-1.5-flash-latest:generateContent?key=YOUR_API_KEY'
		let model_name = &*model_info.model_name;
		let url = match service_type {
			ServiceType::Chat => format!("{url}models/{model_name}:generateContent?key={api_key}"),
			ServiceType::ChatStream => format!("{url}models/{model_name}:streamGenerateContent?key={api_key}"),
		};

		let headers = vec![];

		let GeminiChatRequestParts { system, contents } = Self::into_gemini_request_parts(model_info, chat_req)?;

		let mut payload = json!({
			"contents": contents,
		});

		// Note: It's not clear from the spec if the content of systemInstruction should have a role.
		//       Right now, omitting it (since the spec say it can be only "user" or "model")
		//       It seems to work. https://ai.google.dev/api/rest/v1beta/models/generateContent
		if let Some(system) = system {
			payload.x_insert(
				"systemInstruction",
				json!({
					"parts": [ { "text": system }]
				}),
			)?;
		}

		// -- Add supported ChatOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("/generationConfig/temperature", temperature)?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("/generationConfig/maxOutputTokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("/generationConfig/topP", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(model_info: ModelInfo, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { body, .. } = web_response;

		let gemini_response = Self::body_to_gemini_chat_response(&model_info, body)?;
		let GeminiChatResponse { content, usage } = gemini_response;
		let content = content.map(MessageContent::from);

		Ok(ChatResponse { content, usage })
	}

	fn to_chat_stream(
		model_info: ModelInfo,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let web_stream = WebStream::new_with_pretty_json_array(reqwest_builder);

		let gemini_stream = GeminiStreamer::new(web_stream, model_info, options_set);
		let chat_stream = ChatStream::from_inter_stream(gemini_stream);

		Ok(ChatStreamResponse { stream: chat_stream })
	}
}

// region:    --- Support

/// Support GeminiAdapter functions
impl GeminiAdapter {
	pub(super) fn body_to_gemini_chat_response(model_info: &ModelInfo, mut body: Value) -> Result<GeminiChatResponse> {
		// if the body has a `error` property, then, it is assumed to be an error
		if body.get("error").is_some() {
			return Err(Error::StreamEventError {
				model_info: model_info.clone(),
				body,
			});
		}

		let content = body.x_take::<Value>("/candidates/0/content/parts/0/text")?;
		let usage = body.x_take::<Value>("usageMetadata").map(Self::into_usage).unwrap_or_default();

		Ok(GeminiChatResponse {
			content: content.as_str().map(String::from),
			usage,
		})
	}

	pub(super) fn into_usage(mut usage_value: Value) -> MetaUsage {
		let input_tokens: Option<i32> = usage_value.x_take("promptTokenCount").ok();
		let output_tokens: Option<i32> = usage_value.x_take("candidatesTokenCount").ok();
		let total_tokens: Option<i32> = usage_value.x_take("totalTokenCount").ok();
		MetaUsage {
			input_tokens,
			output_tokens,
			total_tokens,
		}
	}

	/// Takes the genai ChatMessages and build the System string and json Messages for gemini.
	/// - Role mapping `ChatRole:User -> role: "user"`, `ChatRole::Assistant -> role: "model"`
	/// - `ChatRole::System` get concatenated (empty line) into a single `system` for the system instruction.
	///   - This adapter use the v1beta, which supports`systemInstruction`
	/// - the eventual `chat_req.system` get pushed first in the "systemInstruction"
	fn into_gemini_request_parts(model_info: ModelInfo, chat_req: ChatRequest) -> Result<GeminiChatRequestParts> {
		let mut contents: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		// -- Build
		for msg in chat_req.messages {
			// Note: Will handle more types later
			let MessageContent::Text(content) = msg.content;

			match msg.role {
				// for now, system go as "user" (later, we might have adapter_config.system_to_user_impl)
				ChatRole::System => systems.push(content),
				ChatRole::User => contents.push(json! ({"role": "user", "parts": [{"text": content}]})),
				ChatRole::Assistant => contents.push(json! ({"role": "model", "parts": [{"text": content}]})),
				ChatRole::Tool => {
					return Err(Error::MessageRoleNotSupported {
						model_info,
						role: ChatRole::Tool,
					})
				}
			}
		}

		let system = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		Ok(GeminiChatRequestParts { system, contents })
	}
}

// struct Gemini

pub(super) struct GeminiChatResponse {
	pub content: Option<String>,
	pub usage: MetaUsage,
}

struct GeminiChatRequestParts {
	system: Option<String>,
	/// The chat history (user and assistant, except last user message which is message)
	contents: Vec<Value>,
}

// endregion: --- Support
