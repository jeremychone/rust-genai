//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use reqwest::RequestBuilder;

pub struct OllamaAdapter;

const BASE_URL: &str = "http://localhost:11434/v1/";

/// Note: For now, it uses the openai compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base ollama API supports `application/x-ndjson` for streaming whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	fn default_adapter_config(_kind: AdapterKind) -> AdapterConfig {
		AdapterConfig::default()
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(kind, service_type, BASE_URL)
	}

	fn to_web_request_data(
		kind: AdapterKind,
		_config_set: &ConfigSet<'_>,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(kind, model, chat_req, stream, "ollama")
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(kind, web_response)
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStream> {
		OpenAIAdapter::to_chat_stream(kind, reqwest_builder)
	}
}
