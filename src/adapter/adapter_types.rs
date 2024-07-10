use crate::adapter::AdapterConfig;
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use derive_more::Display;
use reqwest::RequestBuilder;
use serde_json::Value;

use super::groq::MODELS as GROQ_MODELS;

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash)]
pub enum AdapterKind {
	OpenAI,
	Ollama,
	Anthropic,
	Cohere,
	Gemini,
	Groq,
	// Note: Variants will probalby be suffixed
	// AnthropicBerock,
}

impl AdapterKind {
	/// Very simplistic mapper for now.
	pub fn from_model(model: &str) -> Result<Self> {
		if model.starts_with("gpt") {
			Ok(AdapterKind::OpenAI)
		} else if model.starts_with("claude") {
			Ok(AdapterKind::Anthropic)
		} else if model.starts_with("command") {
			Ok(AdapterKind::Cohere)
		} else if model.starts_with("gemini") {
			Ok(AdapterKind::Gemini)
		} else if GROQ_MODELS.contains(&model) {
			return Ok(AdapterKind::Groq);
		}
		// for now, fallback on Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

pub(crate) trait Adapter {
	// NOTE: Adapter is a crate Trait, so, ok to use async fn here.
	async fn list_model_names(kind: AdapterKind) -> Result<Vec<String>>;

	// NOTE: Adapter is a crate Trait, so, ok to use async fn here.
	#[deprecated(note = "Use list_model_names(..) (this function will be removed)")]
	#[allow(unused)]
	async fn list_models(kind: AdapterKind) -> Result<Vec<String>> {
		Self::list_model_names(kind).await
	}

	/// The static default AdapterConfig for this AdapterKind
	/// Note: Implementation typically using OnceLock
	fn default_adapter_config(kind: AdapterKind) -> &'static AdapterConfig;

	/// The base service url for this AdapterKind for this given service type.
	/// NOTE: For some services, the url will be further updated in the to_web_request_data
	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String;

	/// To be implemented by Adapters
	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters
	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse>;

	/// To be implemented by Adapters
	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse>;
}

// region:    --- AdapterKind

// endregion: --- AdapterKind

// region:    --- ServiceType

#[derive(Debug, Clone, Copy)]
pub enum ServiceType {
	Chat,
	ChatStream,
}

// endregion: --- ServiceType

// region:    --- WebRequestData

// NOTE: This cannot really move to `webc` bcause it has to be public with the adapter and `webc` is private for now.

pub struct WebRequestData {
	pub url: String,
	pub headers: Vec<(String, String)>,
	pub payload: Value,
}

// endregion: --- WebRequestData
