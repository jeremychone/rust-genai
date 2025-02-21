use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{BatchEmbedRequest, EmbedOptionsSet, EmbedResponse, SingleEmbedRequest};
use crate::webc::WebResponse;
use crate::{ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

use super::groq::GroqAdapter;
use crate::adapter::deepseek::DeepSeekAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::resolver::{AuthData, Endpoint};

/// A construct that allows dispatching calls to the Adapters.
///
/// Note 1: This struct does not need to implement the Adapter trait, as some of its methods take the adapter kind as a parameter.
///
/// Note 2: This struct might be renamed to avoid confusion with the traditional Rust dispatcher pattern.
pub struct AdapterDispatcher;

impl AdapterDispatcher {
	pub fn default_endpoint(kind: AdapterKind) -> Endpoint {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_endpoint(),
			AdapterKind::Anthropic => AnthropicAdapter::default_endpoint(),
			AdapterKind::Cohere => CohereAdapter::default_endpoint(),
			AdapterKind::Ollama => OllamaAdapter::default_endpoint(),
			AdapterKind::Gemini => GeminiAdapter::default_endpoint(),
			AdapterKind::Groq => GroqAdapter::default_endpoint(),
			AdapterKind::Xai => XaiAdapter::default_endpoint(),
			AdapterKind::DeepSeek => DeepSeekAdapter::default_endpoint(),
		}
	}

	pub fn default_auth(kind: AdapterKind) -> AuthData {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_auth(),
			AdapterKind::Anthropic => AnthropicAdapter::default_auth(),
			AdapterKind::Cohere => CohereAdapter::default_auth(),
			AdapterKind::Ollama => OllamaAdapter::default_auth(),
			AdapterKind::Gemini => GeminiAdapter::default_auth(),
			AdapterKind::Groq => GroqAdapter::default_auth(),
			AdapterKind::Xai => XaiAdapter::default_auth(),
			AdapterKind::DeepSeek => DeepSeekAdapter::default_auth(),
		}
	}

	pub async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::all_model_names(kind).await,
			AdapterKind::Anthropic => AnthropicAdapter::all_model_names(kind).await,
			AdapterKind::Cohere => CohereAdapter::all_model_names(kind).await,
			AdapterKind::Ollama => OllamaAdapter::all_model_names(kind).await,
			AdapterKind::Gemini => GeminiAdapter::all_model_names(kind).await,
			AdapterKind::Groq => GroqAdapter::all_model_names(kind).await,
			AdapterKind::Xai => XaiAdapter::all_model_names(kind).await,
			AdapterKind::DeepSeek => DeepSeekAdapter::all_model_names(kind).await,
		}
	}

	pub fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		match model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Cohere => CohereAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Gemini => GeminiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Groq => GroqAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Xai => XaiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::DeepSeek => DeepSeekAdapter::get_service_url(model, service_type, endpoint),
		}
	}

	pub fn get_embed_url(model: &ModelIden, endpoint: Endpoint) -> Option<String> {
		match model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_embed_url(model, endpoint),
			AdapterKind::Anthropic => AnthropicAdapter::get_embed_url(model, endpoint),
			AdapterKind::Cohere => CohereAdapter::get_embed_url(model, endpoint),
			AdapterKind::Ollama => OllamaAdapter::get_embed_url(model, endpoint),
			AdapterKind::Gemini => GeminiAdapter::get_embed_url(model, endpoint),
			AdapterKind::Groq => GroqAdapter::get_embed_url(model, endpoint),
			AdapterKind::Xai => XaiAdapter::get_embed_url(model, endpoint),
			AdapterKind::DeepSeek => DeepSeekAdapter::get_embed_url(model, endpoint),
		}
	}

	pub fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let adapter_kind = &target.model.adapter_kind;
		match adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(target, service_type, chat_req, options_set)
			}
			AdapterKind::Cohere => CohereAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Groq => GroqAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Xai => XaiAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_web_request_data(target, service_type, chat_req, options_set),
		}
	}

	pub fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Xai => XaiAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_chat_response(model_iden, web_response, options_set),
		}
	}

	pub fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Xai => XaiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}

	pub fn embed(
		service_target: ServiceTarget,
		embed_req: SingleEmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		match service_target.model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Cohere => CohereAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Ollama => OllamaAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Gemini => GeminiAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Groq => GroqAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::Xai => XaiAdapter::embed(service_target, embed_req, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::embed(service_target, embed_req, options_set),
		}
	}

	pub fn embed_batch(
		service_target: ServiceTarget,
		embed_req: BatchEmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		match service_target.model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Cohere => CohereAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Ollama => OllamaAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Gemini => GeminiAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Groq => GroqAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::Xai => XaiAdapter::embed_batch(service_target, embed_req, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::embed_batch(service_target, embed_req, options_set),
		}
	}

	pub fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Cohere => CohereAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Groq => GroqAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Xai => XaiAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_embed_response(model_iden, web_response, options_set),
		}
	}
}
