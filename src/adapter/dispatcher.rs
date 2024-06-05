use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::webc::WebResponse;
use crate::Result;
use reqwest_eventsource::EventSource;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	fn default_api_key_env_name(kind: AdapterKind) -> Option<&'static str> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_api_key_env_name(kind),
			AdapterKind::Ollama => OllamaAdapter::default_api_key_env_name(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_api_key_env_name(kind),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(kind, service_type),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(kind, service_type),
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
			AdapterKind::OpenAI => OpenAIAdapter::to_web_request_data(kind, model, chat_req, stream),
			AdapterKind::Ollama => OllamaAdapter::to_web_request_data(kind, model, chat_req, stream),
			AdapterKind::Anthropic => AnthropicAdapter::to_web_request_data(kind, model, chat_req, stream),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(kind, web_response),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(kind, web_response),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(kind, web_response),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_stream(kind: AdapterKind, event_source: EventSource) -> Result<ChatStream> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(kind, event_source),
			AdapterKind::Ollama => OpenAIAdapter::to_chat_stream(kind, event_source),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(kind, event_source),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}
}
