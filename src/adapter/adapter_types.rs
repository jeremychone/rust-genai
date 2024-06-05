use crate::adapter::support::get_api_key_from_config;
use crate::client::ClientConfig;
use crate::webc::WebResponse;
use crate::{ChatRequest, ChatResponse, ChatStream, Result};
use reqwest_eventsource::EventSource;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum AdapterKind {
	OpenAI,
	Ollama,
	Anthropic,
	// -- Not implemented, just to show direction
	Gemini,
	AnthropicBerock,
}

impl AdapterKind {
	/// Very simplistic getter for now.
	pub fn from_model(model: &str) -> Result<Self> {
		if model.starts_with("gpt") {
			Ok(AdapterKind::OpenAI)
		} else if model.contains("claude") {
			Ok(AdapterKind::Anthropic)
		}
		// for now, fallback on Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

pub trait Adapter {
	/// Provide the default api_key environment name by this adapter
	/// default: return None
	fn default_api_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		None
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String;

	/// Get the api_key, with default implementation.
	fn get_api_key(kind: AdapterKind, client_config: Option<&ClientConfig>) -> Result<Option<String>> {
		get_api_key_from_config(client_config, Self::default_api_key_env_name(kind))
	}

	/// To be implemented by Adapters
	fn to_web_request_data(
		kind: AdapterKind,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters
	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse>;

	/// To be implemented by Adapters
	fn to_chat_stream(kind: AdapterKind, event_source: EventSource) -> Result<ChatStream>;
}

// region:    --- AdapterKind

// endregion: --- AdapterKind

// region:    --- ServiceType

pub enum ServiceType {
	Chat,
	ChatStream,
}

// endregion: --- ServiceType

// region:    --- WebRequestData

// NOTE: This cannot really move to `webc` bcause it has to be public with the adapter and `webc` is private for now.

pub struct WebRequestData {
	pub headers: Vec<(String, String)>,
	pub payload: Value,
}

// endregion: --- WebRequestData
