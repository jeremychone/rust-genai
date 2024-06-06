use crate::adapter::support::get_api_key_resolver;
use crate::adapter::AdapterConfig;
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::client::ClientConfig;
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum AdapterKind {
	OpenAI,
	Ollama,
	Anthropic,
	Cohere,
	// -- Not implemented, just to show direction
	Gemini,
	AnthropicBerock,
}

impl AdapterKind {
	/// Very simplistic getter for now.
	pub fn from_model(model: &str) -> Result<Self> {
		if model.starts_with("gpt") {
			Ok(AdapterKind::OpenAI)
		} else if model.starts_with("claude") {
			Ok(AdapterKind::Anthropic)
		} else if model.starts_with("command") {
			Ok(AdapterKind::Cohere)
		}
		// for now, fallback on Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

pub trait Adapter {
	fn default_adapter_config(kind: AdapterKind) -> AdapterConfig;

	fn require_auth(_kind: AdapterKind, config_set: &ConfigSet<'_>) -> bool {
		true
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String;

	/// Get the api_key, with default implementation.
	fn get_api_key(kind: AdapterKind, config_set: &ConfigSet<'_>) -> Result<String> {
		get_api_key_resolver(kind, config_set)
	}

	/// To be implemented by Adapters
	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters
	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse>;

	/// To be implemented by Adapters
	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStream>;
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
