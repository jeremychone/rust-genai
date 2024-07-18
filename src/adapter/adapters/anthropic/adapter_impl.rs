use crate::adapter::anthropic::AnthropicStreamer;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::Result;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{
	ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatRole, ChatStream, ChatStreamResponse, MetaUsage,
};
use crate::utils::x_value::XValue;
use crate::webc::WebResponse;
use crate::ConfigSet;
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub struct AnthropicAdapter;

const BASE_URL: &str = "https://api.anthropic.com/v1/";
const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";
const MODELS: &[&str] = &[
	"claude-3-5-sonnet-20240620",
	"claude-3-opus-20240229",
	"claude-3-sonnet-20240229",
	"claude-3-haiku-20240307",
];

impl Adapter for AnthropicAdapter {
	/// Note: For now returns the common ones (see above)
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(|| AdapterConfig::default().with_auth_env_name("ANTHROPIC_API_KEY"))
	}

	fn get_service_url(_kind: AdapterKind, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}messages"),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let stream = matches!(service_type, ServiceType::ChatStream);
		let url = Self::get_service_url(kind, service_type);

		// -- api_key (this Adapter requires it)
		let api_key = get_api_key_resolver(kind, config_set)?;

		let headers = vec![
			// headers
			("x-api-key".to_string(), api_key.to_string()),
			("anthropic-version".to_string(), ANTRHOPIC_VERSION.to_string()),
		];

		let AnthropicsRequestParts { system, messages } = Self::into_anthropic_request_parts(chat_req)?;

		// -- Build the basic payload
		let mut payload = json!({
			"model": model,
			"messages": messages,
			"stream": stream
		});
		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		// -- Add supported ChatRequestOptions
		if let Some(temperature) = options_set.temperature() {
			payload.x_insert("temperature", temperature)?;
		}

		let max_tokens = options_set.max_tokens().unwrap_or(MAX_TOKENS);
		payload.x_insert("max_tokens", max_tokens)?; // required for anyhropic

		if let Some(top_p) = options_set.top_p() {
			payload.x_insert("top_p", top_p)?;
		}

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(_kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;
		let json_content_items: Vec<Value> = body.x_take("content")?;

		let mut content: Vec<String> = Vec::new();

		let usage = body.x_take("usage").map(Self::into_usage).unwrap_or_default();

		for mut item in json_content_items {
			let item_text: String = item.x_take("text")?;
			content.push(item_text);
		}

		let content = if content.is_empty() {
			None
		} else {
			Some(content.join(""))
		};

		Ok(ChatResponse { content, usage })
	}

	fn to_chat_stream(
		_kind: AdapterKind,
		reqwest_builder: RequestBuilder,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let event_source = EventSource::new(reqwest_builder)?;
		let anthropic_stream = AnthropicStreamer::new(event_source, options_set);
		let chat_stream = ChatStream::from_inter_stream(anthropic_stream);
		Ok(ChatStreamResponse { stream: chat_stream })
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

	/// Takes the genai ChatMessages and build the System string and json Messages for Anthropic.
	/// - Will push the `ChatRequest.system` and systems message to `AnthropicsRequestParts.system`
	fn into_anthropic_request_parts(chat_req: ChatRequest) -> Result<AnthropicsRequestParts> {
		let mut messages: Vec<Value> = Vec::new();
		let mut systems: Vec<String> = Vec::new();

		if let Some(system) = chat_req.system {
			systems.push(system);
		}

		for msg in chat_req.messages {
			let content = msg.content;
			match msg.role {
				// for now, system and tool goes to system
				ChatRole::System | ChatRole::Tool => systems.push(content),
				ChatRole::User => messages.push(json! ({"role": "user", "content": content})),
				ChatRole::Assistant => messages.push(json! ({"role": "assistant", "content": content})),
			}
		}

		let system = if !systems.is_empty() {
			Some(systems.join("\n"))
		} else {
			None
		};

		Ok(AnthropicsRequestParts { system, messages })
	}
}

struct AnthropicsRequestParts {
	system: Option<String>,
	messages: Vec<Value>,
	// TODO: need to add tools
}

// endregion: --- Support
