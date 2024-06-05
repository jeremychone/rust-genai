//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::webc::WebResponse;
use crate::Result;
use reqwest::RequestBuilder;
use reqwest_eventsource::EventSource;

pub struct OllamaAdapter;

const BASE_URL: &str = "http://localhost:11434/v1/";

/// Note: For now, it uses the openai compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base ollama API supports `application/x-ndjson` for streaming whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	fn default_api_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		None
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(kind, service_type, BASE_URL)
	}

	fn to_web_request_data(
		kind: AdapterKind,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(kind, model, chat_req, stream, "ollama".to_string())
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(kind, web_response)
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStream> {
		OpenAIAdapter::to_chat_stream(kind, reqwest_builder)
	}
}

// region:    --- Support

// endregion: --- Support
