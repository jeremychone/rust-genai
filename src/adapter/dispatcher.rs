use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::webc::WebResponse;
use crate::{ChatRequest, ChatResponse};
use crate::{ChatStream, Result};
use reqwest_eventsource::EventSource;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	fn default_api_key_env_name(kind: AdapterKind) -> Option<&'static str> {
		match kind {
			AdapterKind::OpenAI => todo!(),
			AdapterKind::Ollama => todo!(),
			AdapterKind::Anthropic => AnthropicAdapter::default_api_key_env_name(kind),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		match kind {
			AdapterKind::OpenAI => todo!(),
			AdapterKind::Ollama => todo!(),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(kind, service_type),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		match kind {
			AdapterKind::OpenAI => todo!(),
			AdapterKind::Ollama => todo!(),
			AdapterKind::Anthropic => AnthropicAdapter::to_web_request_data(kind, model, chat_req, stream),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		match kind {
			AdapterKind::OpenAI => todo!(),
			AdapterKind::Ollama => todo!(),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(kind, web_response),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_stream(kind: AdapterKind, event_source: EventSource) -> Result<ChatStream> {
		match kind {
			AdapterKind::OpenAI => todo!(),
			AdapterKind::Ollama => todo!(),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(kind, event_source),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}
}
