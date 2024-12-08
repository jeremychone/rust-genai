use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ClientConfig, ModelIden};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

use super::groq::GroqAdapter;
use crate::resolver::{AuthData, Endpoint};

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	fn default_endpoint(kind: AdapterKind) -> Endpoint {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_endpoint(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_endpoint(kind),
			AdapterKind::Cohere => CohereAdapter::default_endpoint(kind),
			AdapterKind::Ollama => OllamaAdapter::default_endpoint(kind),
			AdapterKind::Gemini => GeminiAdapter::default_endpoint(kind),
			AdapterKind::Groq => GroqAdapter::default_endpoint(kind),
		}
	}

	fn default_auth(kind: AdapterKind) -> AuthData {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_auth(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_auth(kind),
			AdapterKind::Cohere => CohereAdapter::default_auth(kind),
			AdapterKind::Ollama => OllamaAdapter::default_auth(kind),
			AdapterKind::Gemini => GeminiAdapter::default_auth(kind),
			AdapterKind::Groq => GroqAdapter::default_auth(kind),
		}
	}

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

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		match model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Cohere => CohereAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Gemini => GeminiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Groq => GroqAdapter::get_service_url(model, service_type, endpoint),
		}
	}

	fn to_web_request_data(
		target: ServiceTarget,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let adapter_kind = &target.model.adapter_kind;
		match adapter_kind {
			AdapterKind::OpenAI => {
				OpenAIAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Cohere => {
				CohereAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Ollama => {
				OllamaAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Gemini => {
				GeminiAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Groq => {
				GroqAdapter::to_web_request_data(target, client_config, service_type, chat_req, options_set)
			}
		}
	}

	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(model_iden, web_response),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response),
			AdapterKind::Cohere => CohereAdapter::to_chat_response(model_iden, web_response),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(model_iden, web_response),
			AdapterKind::Gemini => GeminiAdapter::to_chat_response(model_iden, web_response),
			AdapterKind::Groq => GroqAdapter::to_chat_response(model_iden, web_response),
		}
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Ollama => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}
}
