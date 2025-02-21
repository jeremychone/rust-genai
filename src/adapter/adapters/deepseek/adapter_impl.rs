use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct DeepSeekAdapter;

pub(in crate::adapter) const MODELS: &[&str] = &["deepseek-chat", "deepseek-reasoner"];

impl DeepSeekAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "DEEPSEEK_API_KEY";
}

// The DeepSeek API adapter is modeled after the OpenAI adapter, as the DeepSeek API is compatible with the OpenAI API.
impl Adapter for DeepSeekAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.deepseek.com/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	/// DeepSeek does not support embedding
	fn get_embed_url(_: &ModelIden, _: Endpoint) -> Option<String> {
		None
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options)
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

	fn embed(
		service_target: ServiceTarget,
		_: crate::embed::SingleEmbedRequest,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::EmbeddingNotSupported {
			model_iden: service_target.model,
		})
	}

	fn embed_batch(
		service_target: ServiceTarget,
		_: crate::embed::BatchEmbedRequest,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::EmbeddingNotSupported {
			model_iden: service_target.model,
		})
	}

	fn to_embed_response(
		model_iden: ModelIden,
		_: WebResponse,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::EmbeddingNotSupported { model_iden })
	}
}
