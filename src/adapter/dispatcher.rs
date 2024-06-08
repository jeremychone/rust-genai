use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use reqwest::RequestBuilder;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	fn default_adapter_config(kind: AdapterKind) -> &'static AdapterConfig {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_adapter_config(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_adapter_config(kind),
			AdapterKind::Cohere => CohereAdapter::default_adapter_config(kind),
			AdapterKind::Ollama => OllamaAdapter::default_adapter_config(kind),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(kind, service_type),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(kind, service_type),
			AdapterKind::Cohere => CohereAdapter::get_service_url(kind, service_type),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(kind, service_type),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		model: &str,
		chat_req: ChatRequest,
		stream: bool,
	) -> Result<WebRequestData> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_web_request_data(kind, config_set, model, chat_req, stream),
			AdapterKind::Anthropic => AnthropicAdapter::to_web_request_data(kind, config_set, model, chat_req, stream),
			AdapterKind::Cohere => CohereAdapter::to_web_request_data(kind, config_set, model, chat_req, stream),
			AdapterKind::Ollama => OllamaAdapter::to_web_request_data(kind, config_set, model, chat_req, stream),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(kind, web_response),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(kind, web_response),
			AdapterKind::Cohere => CohereAdapter::to_chat_response(kind, web_response),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(kind, web_response),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Ollama => OpenAIAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Gemini => todo!(),
			AdapterKind::AnthropicBerock => todo!(),
		}
	}
}
