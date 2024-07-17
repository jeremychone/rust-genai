use crate::adapter::openai::OpenAIMessagesStream;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatRole, ChatStream, ChatStreamResponse};
use crate::utils::x_value::XValue;
use crate::webc::WebResponse;
use crate::{ConfigSet, Error, Result};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub struct OpenAIAdapter;

const BASE_URL: &str = "https://api.openai.com/v1/";
const MODELS: &[&str] = &["gpt-4o", "gpt-4-turbo", "gpt-4", "gpt-3.5-turbo"];

impl Adapter for OpenAIAdapter {
	/// Note: For now returns the common ones (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(|| AdapterConfig::default().with_auth_env_name("OPENAI_API_KEY"))
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		Self::util_get_service_url(kind, service_type, BASE_URL)
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		chat_req_options: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		// -- api_key (this Adapter requires it)
		let api_key = get_api_key_resolver(kind, config_set)?;
		let url = Self::get_service_url(kind, service_type);

		OpenAIAdapter::util_to_web_request_data(
			kind,
			url,
			model,
			chat_req,
			service_type,
			chat_req_options,
			&api_key,
			false,
		)
	}

	fn to_chat_response(_kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;

		let first_choice: Option<Value> = body.x_take("/choices/0")?;
		let content: Option<String> = first_choice.map(|mut c| c.x_take("/message/content")).transpose()?;
		Ok(ChatResponse {
			content,
			..Default::default()
		})
	}

	fn to_chat_stream(
		_kind: AdapterKind,
		reqwest_builder: RequestBuilder,
		options_sets: ChatRequestOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let openai_stream = OpenAIMessagesStream::new(event_source, options_sets);
		let chat_stream = ChatStream::from_inter_stream(openai_stream);

		Ok(ChatStreamResponse { stream: chat_stream })
	}
}

/// Support function for other Adapter that share OpenAI APIs
impl OpenAIAdapter {
	pub(in crate::adapter) fn util_get_service_url(
		_kind: AdapterKind,
		service_type: ServiceType,
		// -- util args
		base_url: &str,
	) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{base_url}chat/completions"),
		}
	}

	#[allow(clippy::too_many_arguments)] // ok because internal only
	pub(in crate::adapter) fn util_to_web_request_data(
		kind: AdapterKind,
		url: String,
		model: &str,
		chat_req: ChatRequest,
		service_type: ServiceType,
		options_set: ChatRequestOptionsSet<'_, '_>,
		// -- utils args
		api_key: &str,
		ollama_variant: bool,
	) -> Result<WebRequestData> {
		let stream = matches!(service_type, ServiceType::ChatStream);

		// -- Build the header
		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		// -- Build the basic payload
		let OpenAIRequestParts { messages } = into_openai_messages(kind, chat_req, ollama_variant)?;
		let mut payload = json!({
			"model": model,
			"messages": messages,
			"stream": stream
		});

		// --
		if stream & options_set.capture_usage().unwrap_or(false) {
			payload.x_insert("stream_options", json!({"include_usage": true}))?;
		}

		// -- Add supported ChatRequestOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("tdsemperature", temperature)?;
		}
		if let Some(max_tokens) = options_set.max_tokens() {
			payload.x_insert("max_tokens", max_tokens)?;
		}
		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}
}

// region:    --- Support

struct OpenAIRequestParts {
	messages: Vec<Value>,
}

/// Takes the genai ChatMessages and build the OpenAIChatRequestParts
/// - `genai::ChatRequest.system`, if present, goes as first message with role 'system'.
/// - All messages get added with the corresponding roles (does not support tools for now)
/// NOTE: here, the last `true` is for the ollama variant
///       It seems the Ollama compaitiblity layer does not work well with multiple System message.
///       So, when `true`, it will concatenate the system message as a single on at the beginning
fn into_openai_messages(
	adapter_kind: AdapterKind,
	chat_req: ChatRequest,
	ollama_variant: bool,
) -> Result<OpenAIRequestParts> {
	let mut system_messages: Vec<String> = Vec::new();
	let mut messages: Vec<Value> = Vec::new();

	if let Some(system_msg) = chat_req.system {
		if ollama_variant {
			system_messages.push(system_msg)
		} else {
			messages.push(json!({"role": "system", "content": system_msg}));
		}
	}

	for chat_msg in chat_req.messages {
		let content = chat_msg.content;
		match chat_msg.role {
			// for now, system and tool goes to system
			ChatRole::System => {
				// see note in the funtion comment
				if ollama_variant {
					system_messages.push(content);
				} else {
					messages.push(json!({"role": "system", "content": content}))
				}
			}
			ChatRole::User => messages.push(json! ({"role": "user", "content": content})),
			ChatRole::Assistant => messages.push(json! ({"role": "assistant", "content": content})),
			ChatRole::Tool => {
				return Err(Error::AdapterMessageRoleNotSupport {
					adapter_kind,
					role: ChatRole::Tool,
				})
			}
		}
	}

	if !system_messages.is_empty() {
		let system_message = system_messages.join("\n");
		messages.insert(0, json!({"role": "system", "content": system_message}));
	}

	Ok(OpenAIRequestParts { messages })
}

// endregion: --- Support
