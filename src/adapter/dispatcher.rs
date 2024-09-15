use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::Result;
use crate::{ClientConfig, ModelIden};
use reqwest::RequestBuilder;

use super::groq::GroqAdapter;

pub struct AdapterDispatcher;

impl Adapter for AdapterDispatcher {
	fn default_key_env_name(kind: AdapterKind) -> Option<&'static str> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_key_env_name(kind),
			AdapterKind::Anthropic => AnthropicAdapter::default_key_env_name(kind),
			AdapterKind::Cohere => CohereAdapter::default_key_env_name(kind),
			AdapterKind::Ollama => OllamaAdapter::default_key_env_name(kind),
			AdapterKind::Gemini => GeminiAdapter::default_key_env_name(kind),
			AdapterKind::Groq => GroqAdapter::default_key_env_name(kind),
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

	fn get_service_url(model_iden: ModelIden, service_type: ServiceType) -> String {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(model_iden, service_type),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(model_iden, service_type),
			AdapterKind::Cohere => CohereAdapter::get_service_url(model_iden, service_type),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(model_iden, service_type),
			AdapterKind::Gemini => GeminiAdapter::get_service_url(model_iden, service_type),
			AdapterKind::Groq => GroqAdapter::get_service_url(model_iden, service_type),
		}
	}

	fn to_web_request_data(
		model_iden: ModelIden,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => {
				OpenAIAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Cohere => {
				CohereAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Ollama => {
				OllamaAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Gemini => {
				GeminiAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
			}
			AdapterKind::Groq => {
				GroqAdapter::to_web_request_data(model_iden, client_config, service_type, chat_req, options_set)
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