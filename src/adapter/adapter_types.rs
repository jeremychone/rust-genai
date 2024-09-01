use crate::adapter::AdapterKind;
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::Result;
use crate::{ClientConfig, ModelIden};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Adapter {
	fn default_key_env_name(kind: AdapterKind) -> Option<&'static str>;

	// NOTE: Adapter is a crate Trait, so, ok to use async fn here.
	async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>>;

	/// The base service url for this AdapterKind for this given service type.
	/// NOTE: For some services, the url will be further updated in the to_web_request_data
	fn get_service_url(model_iden: ModelIden, service_type: ServiceType) -> String;

	/// To be implemented by Adapters
	fn to_web_request_data(
		model_iden: ModelIden,
		config_set: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData>;

	/// To be implemented by Adapters
	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse>;

	/// To be implemented by Adapters
	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse>;
}

// region:    --- ServiceType

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ServiceType {
	Chat,
	ChatStream,
}

// endregion: --- ServiceType

// region:    --- WebRequestData

// NOTE: This cannot really move to `webc` because it has to be public with the adapter and `webc` is private for now.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRequestData {
	pub url: String,
	pub headers: Vec<(String, String)>,
	pub payload: Value,
}

// endregion: --- WebRequestData
