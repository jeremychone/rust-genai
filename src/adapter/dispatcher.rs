use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::ConfigSet;
use crate::Result;
use reqwest::RequestBuilder;

use super::groq::GroqAdapter;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::all_model_names(kind).await,
			AdapterKind::Anthropic => AnthropicAdapter::all_model_names(kind).await,
			AdapterKind::Cohere => CohereAdapter::all_model_names(kind).await,
			AdapterKind::Ollama => OllamaAdapter::all_model_names(kind).await,
			AdapterKind::Gemini => GeminiAdapter::all_model_names(kind).await,
			AdapterKind::Groq => GroqAdapter::all_model_names(kind).await,
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
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		match kind {
			AdapterKind::OpenAI => {
				OpenAIAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
			}
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
			}
			AdapterKind::Cohere => {
				CohereAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
			}
			AdapterKind::Ollama => {
				OllamaAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
			}
			AdapterKind::Gemini => {
				GeminiAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
			}
			AdapterKind::Groq => {
				GroqAdapter::to_web_request_data(kind, config_set, service_type, model, chat_req, options_set)
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

	fn to_chat_stream(
		kind: AdapterKind,
		reqwest_builder: RequestBuilder,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(kind, reqwest_builder, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(kind, reqwest_builder, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(kind, reqwest_builder, options_set),
			AdapterKind::Ollama => OpenAIAdapter::to_chat_stream(kind, reqwest_builder, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_stream(kind, reqwest_builder, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_stream(kind, reqwest_builder, options_set),
		}
	}
}
