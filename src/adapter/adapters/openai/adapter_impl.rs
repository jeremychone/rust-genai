//! API DOC: https://platform.openai.com/docs/api-reference/chat

use crate::adapter::openai::{OpenAIMessagesStream, OpenAIStreamEvent};
use crate::adapter::support::get_api_key_from_config;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatMessage, ChatRequest, ChatResponse, ChatRole, ChatStream, StreamItem};
use crate::utils::x_value::XValue;
use crate::webc::WebResponse;
use crate::{Error, Result};
use futures::StreamExt;
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::{json, Value};

pub struct OpenAIAdapter;

const BASE_URL: &str = "https://api.openai.com/v1/";

impl Adapter for OpenAIAdapter {
	fn default_api_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("OPENAI_API_KEY")
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		Self::util_get_service_url(kind, service_type, BASE_URL)
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
		OpenAIAdapter::util_to_web_request_data(kind, model, chat_req, stream, api_key)
	}

	fn to_chat_response(_kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		let WebResponse { mut body, .. } = web_response;
		let first_choice: Option<Value> = body.x_take("/choices/0")?;
		let content: Option<String> = first_choice.map(|mut c| c.x_take("/message/content")).transpose()?;
		Ok(ChatResponse { content })
	}

	fn to_chat_stream(_kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStream> {
		let event_source = EventSource::new(reqwest_builder)?;

		let openai_stream = OpenAIMessagesStream::new(event_source);
		let stream = openai_stream.filter_map(|an_stream_event| async move {
			match an_stream_event {
				Ok(OpenAIStreamEvent::Chunck(content)) => Some(Ok(StreamItem { content: Some(content) })),
				Err(err) => Some(Err(err)),
				_ => None,
			}
		});
		Ok(ChatStream {
			stream: Box::pin(stream),
		})
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
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
		// -- utils args
		api_key: String,
	) -> Result<WebRequestData> {
		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let messages = into_openai_messages(kind, chat_req.messages)?;
		let payload = json!({
			"model": model,
			"messages": messages,
			"stream": stream
		});

		Ok(WebRequestData { headers, payload })
	}
}

// region:    --- Support

/// Takes the genai ChatMessages and build the System string and json Messages for Anthropic.
/// NOTE: Here we do not use serde serialization as we might want to use the annotations for other purpose later.
pub(in crate::adapter) fn into_openai_messages(
	adapter_kind: AdapterKind,
	chat_msgs: Vec<ChatMessage>,
) -> Result<Vec<Value>> {
	let mut messages: Vec<Value> = Vec::new();

	for chat_msg in chat_msgs {
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

	Ok(messages)
}

// endregion: --- Support
