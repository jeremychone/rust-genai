use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptions, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};
use reqwest::RequestBuilder;

use super::groq::GroqAdapter;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	async fn list_models(kind: AdapterKind) -> Result<Vec<String>> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::list_models(kind).await,
			AdapterKind::Anthropic => AnthropicAdapter::list_models(kind).await,
			AdapterKind::Cohere => CohereAdapter::list_models(kind).await,
			AdapterKind::Ollama => OllamaAdapter::list_models(kind).await,
			AdapterKind::Gemini => GeminiAdapter::list_models(kind).await,
			AdapterKind::Groq => GroqAdapter::list_models(kind).await,
		}
	}

	fn default_adapter_config(kind: AdapterKind) -> &'static AdapterConfig {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_adapter_config(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_adapter_config(kind),
			AdapterKind::Cohere => CohereAdapter::default_adapter_config(kind),
			AdapterKind::Ollama => OllamaAdapter::default_adapter_config(kind),
			AdapterKind::Gemini => GeminiAdapter::default_adapter_config(kind),
			AdapterKind::Groq => GroqAdapter::default_adapter_config(kind),
		}
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(kind, service_type),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(kind, service_type),
			AdapterKind::Cohere => CohereAdapter::get_service_url(kind, service_type),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(kind, service_type),
			AdapterKind::Gemini => GeminiAdapter::get_service_url(kind, service_type),
			AdapterKind::Groq => GroqAdapter::get_service_url(kind, service_type),
		}
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		chat_req_options: Option<&ChatRequestOptions>,
	) -> Result<WebRequestData> {
		match kind {
			AdapterKind::OpenAI => {
				OpenAIAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
			AdapterKind::Cohere => {
				CohereAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
			AdapterKind::Ollama => {
				OllamaAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
			AdapterKind::Gemini => {
				GeminiAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
			AdapterKind::Groq => {
				GroqAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, chat_req_options)
			}
		}
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(kind, web_response),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(kind, web_response),
			AdapterKind::Cohere => CohereAdapter::to_chat_response(kind, web_response),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(kind, web_response),
			AdapterKind::Gemini => GeminiAdapter::to_chat_response(kind, web_response),
			AdapterKind::Groq => GroqAdapter::to_chat_response(kind, web_response),
		}
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Ollama => OpenAIAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Gemini => GeminiAdapter::to_chat_stream(kind, reqwest_builder),
			AdapterKind::Groq => GroqAdapter::to_chat_stream(kind, reqwest_builder),
		}
	}
}
