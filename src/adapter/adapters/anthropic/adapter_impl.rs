use crate::adapter::anthropic::{AnthropicMessagesStream, AnthropicStreamEvent};
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatRole, ChatStream, StreamItem};
use crate::utils::x_value::XValue;
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use futures::StreamExt;
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};

pub struct AnthropicAdapter;

const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";
const BASE_URL: &str = "https://api.anthropic.com/v1/";

impl Adapter for AnthropicAdapter {
	fn default_adapter_config(_kind: AdapterKind) -> AdapterConfig {
		AdapterConfig::default().with_auth_env_name("ANTHROPIC_API_KEY")
	}

	fn get_service_url(_kind: AdapterKind, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}messages"),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		// -- api_key (this Adapter requires it)
		let api_key = get_api_key_resolver(kind, config_set)?;

		let headers = vec![
			// headers
			("x-api-key".to_string(), api_key.to_string()),
			("anthropic-version".to_string(), ANTRHOPIC_VERSION.to_string()),
		];

		let AnthropicsRequestParts { system, messages } = into_anthropic_request_parts(chat_req)?;
		let mut payload = json!({
			"model": model,
			"max_tokens": MAX_TOKENS,
			"messages": messages,
			"stream": stream
		});
		if let Some(system) = system {
			payload.x_insert("system", system)?;
		}

		Ok(WebRequestData { headers, payload })
	}

	fn to_chat_response(_kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;
		let json_content_items: Vec<Value> = body.x_take("content")?;

		let mut content: Vec<String> = Vec::new();

		for mut item in json_content_items {
			let item_text: String = item.x_take("text")?;
			content.push(item_text);
		}

		let content = if content.is_empty() {
			None
		} else {
			Some(content.join(""))
		};

		Ok(ChatResponse { content })
	}

	fn to_chat_stream(_kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStream> {
		let event_source = EventSource::new(reqwest_builder)?;
		let anthropic_stream = AnthropicMessagesStream::new(event_source);
		let stream = anthropic_stream.filter_map(|an_stream_event| async move {
			match an_stream_event {
				Err(err) => Some(Err(err)),
				Ok(AnthropicStreamEvent::BlockDelta(content)) => Some(Ok(StreamItem { content: Some(content) })),
				_ => None,
			}
		});
		Ok(ChatStream {
			stream: Box::pin(stream),
		})
	}
}

// region:    --- Support

struct AnthropicsRequestParts {
	system: Option<String>,
	messages: Vec<Value>,
	// TODO: need to add tools
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

// endregion: --- Support
