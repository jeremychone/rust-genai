use crate::adapter::Result;
use crate::adapter::{AdapterConfig, AdapterKind};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::ConfigSet;
use reqwest::RequestBuilder;
use serde_json::Value;

pub trait Adapter {
	// NOTE: Adapter is a crate Trait, so, ok to use async fn here.
	async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>>;

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
	fn to_chat_stream(
		kind: AdapterKind,
		reqwest_builder: RequestBuilder,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse>;
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
