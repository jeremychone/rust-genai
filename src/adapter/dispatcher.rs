use super::groq::GroqAdapter;
use crate::adapter::adapters::mimo::MimoAdapter;
use crate::adapter::adapters::together::TogetherAdapter;
use crate::adapter::adapters::zai::ZaiAdapter;
use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::bigmodel::BigModelAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::deepseek::DeepSeekAdapter;
use crate::adapter::fireworks::FireworksAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::nebius::NebiusAdapter;
use crate::adapter::ollama::OllamaAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::openai_resp::OpenAIRespAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, ModelIden};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

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
			AdapterKind::OpenAIResp => OpenAIRespAdapter::default_endpoint(),
			AdapterKind::Gemini => GeminiAdapter::default_endpoint(),
			AdapterKind::Anthropic => AnthropicAdapter::default_endpoint(),
			AdapterKind::Fireworks => FireworksAdapter::default_endpoint(),
			AdapterKind::Together => TogetherAdapter::default_endpoint(),
			AdapterKind::Groq => GroqAdapter::default_endpoint(),
			AdapterKind::Mimo => MimoAdapter::default_endpoint(),
			AdapterKind::Nebius => NebiusAdapter::default_endpoint(),
			AdapterKind::Xai => XaiAdapter::default_endpoint(),
			AdapterKind::DeepSeek => DeepSeekAdapter::default_endpoint(),
			AdapterKind::Zai => ZaiAdapter::default_endpoint(),
			AdapterKind::BigModel => BigModelAdapter::default_endpoint(),
			AdapterKind::Cohere => CohereAdapter::default_endpoint(),
			AdapterKind::Ollama => OllamaAdapter::default_endpoint(),
		}
	}

	pub fn default_auth(kind: AdapterKind) -> AuthData {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::default_auth(),
			AdapterKind::OpenAIResp => OpenAIRespAdapter::default_auth(),
			AdapterKind::Gemini => GeminiAdapter::default_auth(),
			AdapterKind::Anthropic => AnthropicAdapter::default_auth(),
			AdapterKind::Fireworks => FireworksAdapter::default_auth(),
			AdapterKind::Together => TogetherAdapter::default_auth(),
			AdapterKind::Groq => GroqAdapter::default_auth(),
			AdapterKind::Mimo => MimoAdapter::default_auth(),
			AdapterKind::Nebius => NebiusAdapter::default_auth(),
			AdapterKind::Xai => XaiAdapter::default_auth(),
			AdapterKind::DeepSeek => DeepSeekAdapter::default_auth(),
			AdapterKind::Zai => ZaiAdapter::default_auth(),
			AdapterKind::BigModel => BigModelAdapter::default_auth(),
			AdapterKind::Cohere => CohereAdapter::default_auth(),
			AdapterKind::Ollama => OllamaAdapter::default_auth(),
		}
	}

	pub async fn all_model_names(kind: AdapterKind) -> Result<Vec<String>> {
		match kind {
			AdapterKind::OpenAI => OpenAIAdapter::all_model_names(kind).await,
			AdapterKind::OpenAIResp => OpenAIRespAdapter::all_model_names(kind).await,
			AdapterKind::Gemini => GeminiAdapter::all_model_names(kind).await,
			AdapterKind::Anthropic => AnthropicAdapter::all_model_names(kind).await,
			AdapterKind::Fireworks => FireworksAdapter::all_model_names(kind).await,
			AdapterKind::Together => TogetherAdapter::all_model_names(kind).await,
			AdapterKind::Groq => GroqAdapter::all_model_names(kind).await,
			AdapterKind::Mimo => MimoAdapter::all_model_names(kind).await,
			AdapterKind::Nebius => NebiusAdapter::all_model_names(kind).await,
			AdapterKind::Xai => XaiAdapter::all_model_names(kind).await,
			AdapterKind::DeepSeek => DeepSeekAdapter::all_model_names(kind).await,
			AdapterKind::Zai => ZaiAdapter::all_model_names(kind).await,
			AdapterKind::BigModel => BigModelAdapter::all_model_names(kind).await,
			AdapterKind::Cohere => CohereAdapter::all_model_names(kind).await,
			AdapterKind::Ollama => OllamaAdapter::all_model_names(kind).await,
		}
	}

	pub fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		match model.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::OpenAIResp => OpenAIRespAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Gemini => GeminiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Anthropic => AnthropicAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Fireworks => FireworksAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Together => TogetherAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Groq => GroqAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Mimo => MimoAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Nebius => NebiusAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Xai => XaiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::DeepSeek => DeepSeekAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Zai => ZaiAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::BigModel => BigModelAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Cohere => CohereAdapter::get_service_url(model, service_type, endpoint),
			AdapterKind::Ollama => OllamaAdapter::get_service_url(model, service_type, endpoint),
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
			AdapterKind::OpenAIResp => {
				OpenAIRespAdapter::to_web_request_data(target, service_type, chat_req, options_set)
			}
			AdapterKind::Gemini => GeminiAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Anthropic => {
				AnthropicAdapter::to_web_request_data(target, service_type, chat_req, options_set)
			}
			AdapterKind::Fireworks => {
				FireworksAdapter::to_web_request_data(target, service_type, chat_req, options_set)
			}
			AdapterKind::Together => TogetherAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Groq => GroqAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Mimo => MimoAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Nebius => NebiusAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Xai => XaiAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Zai => ZaiAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::BigModel => BigModelAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Cohere => CohereAdapter::to_web_request_data(target, service_type, chat_req, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_web_request_data(target, service_type, chat_req, options_set),
		}
	}

	pub fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::OpenAIResp => OpenAIRespAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Fireworks => FireworksAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Together => TogetherAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Mimo => MimoAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Nebius => NebiusAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Xai => XaiAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Zai => ZaiAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::BigModel => BigModelAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_response(model_iden, web_response, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_chat_response(model_iden, web_response, options_set),
		}
	}

	pub fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::OpenAIResp => OpenAIRespAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Gemini => GeminiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Fireworks => FireworksAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Together => TogetherAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Groq => GroqAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Mimo => MimoAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Nebius => NebiusAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Xai => XaiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Zai => ZaiAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::BigModel => BigModelAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Cohere => CohereAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}

	pub fn to_embed_request_data(
		target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let adapter_kind = &target.model.adapter_kind;
		match adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::OpenAIResp => Err(Error::AdapterNotSupported {
				adapter_kind: target.model.adapter_kind,
				feature: "embed".to_string(),
			}),
			AdapterKind::Gemini => GeminiAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Fireworks => FireworksAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Together => TogetherAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Groq => GroqAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Mimo => MimoAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Nebius => NebiusAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Xai => XaiAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Zai => ZaiAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::BigModel => BigModelAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Cohere => CohereAdapter::to_embed_request_data(target, embed_req, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_embed_request_data(target, embed_req, options_set),
		}
	}

	pub fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		match model_iden.adapter_kind {
			AdapterKind::OpenAI => OpenAIAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::OpenAIResp => Err(Error::AdapterNotSupported {
				adapter_kind: model_iden.adapter_kind,
				feature: "embed".to_string(),
			}),
			AdapterKind::Gemini => GeminiAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Anthropic => AnthropicAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Fireworks => FireworksAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Together => TogetherAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Groq => GroqAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Mimo => MimoAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Nebius => NebiusAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Xai => XaiAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::DeepSeek => DeepSeekAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Zai => ZaiAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::BigModel => BigModelAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Cohere => CohereAdapter::to_embed_response(model_iden, web_response, options_set),
			AdapterKind::Ollama => OllamaAdapter::to_embed_response(model_iden, web_response, options_set),
		}
	}
}
