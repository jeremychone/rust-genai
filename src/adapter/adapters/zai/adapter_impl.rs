use crate::ModelIden;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct ZAiAdapter;

// Z.AI model names
// Based on available information, Z.AI supports GLM models
// Note: This list may need updating based on actual Z.AI API documentation
pub(in crate::adapter) const MODELS: &[&str] = &[
	"glm-4.6",
	"glm-4",
	"glm-3-turbo",
	// If Z.AI also supports Claude models, they would be listed here
	// "claude-3-5-sonnet-20241022",
	// "claude-3-5-haiku-20241022",
];

impl ZAiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "ZAI_API_KEY";
}

// Z.AI adapter uses OpenAI-compatible implementation (most common format)
// Note: This may need adjustment based on actual Z.AI API documentation
impl Adapter for ZAiAdapter {
	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.z.ai/v1/";
		Endpoint::from_static(BASE_URL)
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
	}

	fn to_embed_request_data(
		target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::to_embed_request_data(target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
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
}
