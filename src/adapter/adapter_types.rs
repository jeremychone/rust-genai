use crate::adapter::AdapterKind;
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{ClientConfig, ModelIden};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::Value;

pub trait Adapter {
	// #[deprecated(note = "use default_auth")]
	// fn default_key_env_name(kind: AdapterKind) -> Option<&'static str>;

	fn default_auth(kind: AdapterKind) -> AuthData;

	fn default_endpoint(kind: AdapterKind) -> Endpoint;

	// NOTE: Adapter is a crate trait, so it is acceptable to use async fn here.
	async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>>;

	/// The base service URL for this AdapterKind for the given service type.
	/// NOTE: For some services, the URL will be further updated in the to_web_request_data method.
	fn get_service_url(model_iden: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String;

	/// To be implemented by Adapters.
	fn to_web_request_data(
		service_target: ServiceTarget,
		config_set: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters.
	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse>;

	/// To be implemented by Adapters.
	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse>;
}

// region:    --- ServiceType

#[derive(Debug, Clone, Copy)]
pub enum ServiceType {
	Chat,
	ChatStream,
}

// endregion: --- ServiceType

// region:    --- WebRequestData

// NOTE: This cannot really move to `webc` because it must be public with the adapter, and `webc` is private for now.
#[derive(Debug, Clone)]
pub struct WebRequestData {
	pub url: String,
	pub headers: Vec<(String, String)>,
	pub payload: Value,
}

// endregion: --- WebRequestData
