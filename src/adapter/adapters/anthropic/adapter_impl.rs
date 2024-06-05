//! API DOC: https://docs.anthropic.com/en/api/messages

use crate::adapter::anthropic::{AnthropicMessagesStream, AnthropicStreamEvent};
use crate::adapter::support::get_api_key_from_config;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatMessage, ChatRequest, ChatResponse, ChatRole, ChatStream, StreamItem};
use crate::utils::x_value::XValue;
use crate::webc::WebResponse;
use crate::{Error, Result};
use futures::StreamExt;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};

pub struct AnthropicAdapter;

const MAX_TOKENS: u32 = 1024;
const ANTRHOPIC_VERSION: &str = "2023-06-01";
const BASE_URL: &str = "https://api.anthropic.com/v1/";

// see: https://docs.anthropic.com/en/api/messages
impl Adapter for AnthropicAdapter {
	fn default_api_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("ANTHROPIC_API_KEY")
	}

	fn get_service_url(_kind: AdapterKind, service_type: ServiceType) -> String {
		match service_type {
			ServiceType::Chat | ServiceType::ChatStream => format!("{BASE_URL}messages"),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		// -- api_key (this Adapter requires it)
		let Some(api_key) = get_api_key_from_config(None, Self::default_api_key_env_name(kind))? else {
			return Err(Error::AdapterRequiresApiKey { adapter_kind: kind });
		};

		let headers = vec![
			// headers
			("x-api-key".to_string(), api_key.to_string()),
			("anthropic-version".to_string(), ANTRHOPIC_VERSION.to_string()),
		];

		let AnthropicsRequestParts { system, messages } = into_anthropic_request_parts(chat_req.messages)?;
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

	fn to_chat_stream(_kind: AdapterKind, event_source: EventSource) -> Result<ChatStream> {
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
/// NOTE: Here we do not use serde serialization as we might want to use the annotations for other purpose later.
fn into_anthropic_request_parts(msgs: Vec<ChatMessage>) -> Result<AnthropicsRequestParts> {
	let mut messages: Vec<Value> = Vec::new();
	let mut systems: Vec<String> = Vec::new();

	for msg in msgs {
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
