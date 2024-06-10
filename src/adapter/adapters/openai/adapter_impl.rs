use crate::adapter::openai::OpenAIMessagesStream;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptions, ChatResponse, ChatRole, ChatStream, ChatStreamResponse};
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
	async fn list_models(_kind: AdapterKind) -> Result<Vec<String>> {
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
		_chat_req_options: Option<&ChatRequestOptions>,
	) -> Result<WebRequestData> {
		// -- api_key (this Adapter requires it)
		let api_key = get_api_key_resolver(kind, config_set)?;
		let url = Self::get_service_url(kind, service_type);

		OpenAIAdapter::util_to_web_request_data(kind, url, model, chat_req, service_type, &api_key)
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

	fn to_chat_stream(_kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let openai_stream = OpenAIMessagesStream::new(event_source);
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

	pub(in crate::adapter) fn util_to_web_request_data(
		kind: AdapterKind,
		url: String,
		model: &str,
		chat_req: ChatRequest,
		service_type: ServiceType,
		// -- utils args
		api_key: &str,
	) -> Result<WebRequestData> {
		let stream = matches!(service_type, ServiceType::ChatStream);

		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let OpenAIRequestParts { messages } = into_openai_messages(kind, chat_req)?;
		let payload = json!({
			"model": model,
			"messages": messages,
			"stream": stream
		});

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
/// -
fn into_openai_messages(adapter_kind: AdapterKind, chat_req: ChatRequest) -> Result<OpenAIRequestParts> {
	let mut messages: Vec<Value> = Vec::new();

	if let Some(system_msg) = chat_req.system {
		messages.push(json!({"role": "system", "content": system_msg}));
	}

	for chat_msg in chat_req.messages {
		let content = chat_msg.content;
		match chat_msg.role {
			// for now, system and tool goes to system
			ChatRole::System => messages.push(json!({"role": "system", "content": content})),
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

	Ok(OpenAIRequestParts { messages })
}

// endregion: --- Support
