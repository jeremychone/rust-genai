use crate::ServiceTarget;
use crate::adapter::adapters::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Headers, ModelIden, Result};
use reqwest::RequestBuilder;

pub struct OpenRouterAdapter;

impl OpenRouterAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &'static str = "OPENROUTER_API_KEY";

	/// Add OpenRouter-specific headers to the request
	fn add_openrouter_headers(headers: Headers) -> Headers {
		let openrouter_headers = Headers::from([
			("HTTP-Referer".to_string(), "https://github.com/sst/genai".to_string()),
			("X-Title".to_string(), "genai-rust".to_string()),
		]);
		openrouter_headers.applied_to(headers)
	}
}

impl Adapter for OpenRouterAdapter {
	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://openrouter.ai/api/v1/";
		Endpoint::from_static(BASE_URL)
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		// For now, return empty - OpenRouter has many models and they should be specified directly
		Ok(vec![])
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let mut web_request_data = OpenAIAdapter::to_web_request_data(target, service_type, chat_req, chat_options)?;

		// Add OpenRouter-specific headers
		web_request_data.headers = Self::add_openrouter_headers(web_request_data.headers);

		Ok(web_request_data)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		// For now, OpenRouter embeddings are not supported
		// This would require access to the private embed module in openai
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: AdapterKind::OpenRouter,
			feature: "embed".to_string(),
		})
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, _options_set)
	}
}
