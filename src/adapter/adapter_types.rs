use crate::adapter::support::get_api_key_resolver;
use crate::adapter::AdapterConfig;
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use derive_more::Display;
use reqwest::RequestBuilder;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash)]
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
	/// The static default AdapterConfig for this AdapterKind
	/// Note: Implementation typically using OnceLock
	fn default_adapter_config(kind: AdapterKind) -> &'static AdapterConfig;

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
