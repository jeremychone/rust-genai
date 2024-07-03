//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use reqwest::RequestBuilder;
use std::sync::OnceLock;

pub struct OllamaAdapter;

const BASE_URL: &str = "http://localhost:11434/v1/";

/// Note: For now, it uses the openai compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base ollama API supports `application/x-ndjson` for streaming whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	/// Note: For now returns empty as it should probably do a request to the ollama server
	async fn list_models(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(Vec::new())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(AdapterConfig::default)
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(kind, service_type, BASE_URL)
	}

	fn to_web_request_data(
		kind: AdapterKind,
		_config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let url = Self::get_service_url(kind, service_type);

		OpenAIAdapter::util_to_web_request_data(kind, url, model, chat_req, service_type, options_set, "ollama", true)
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(kind, web_response)
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(kind, reqwest_builder)
	}
}
