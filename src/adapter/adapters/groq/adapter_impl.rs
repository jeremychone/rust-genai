use crate::ModelIden;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct GroqAdapter;

// ~ newer on top when/if possible
pub(in crate::adapter) const MODELS: &[&str] = &[
	"moonshotai/kimi-k2-instruct",
	"qwen/qwen3-32b",
	"mistral-saba-24b",
	"meta-llama/llama-4-scout-17b-16e-instruct",
	"meta-llama/llama-4-maverick-17b-128e-instruct",
	"llama-3.3-70b-versatile",
	"llama-3.2-3b-preview",
	"llama-3.2-1b-preview",
	"llama-3.1-405b-reasoning",
	"llama-3.1-70b-versatile",
	"llama-3.1-8b-instant",
	"mixtral-8x7b-32768",
	"gemma2-9b-it",
	"gemma-7b-it", // deprecated
	"llama-3.1-8b-instant",
	"llama-guard-3-8b",
	"llama3-70b-8192",
	// -- preview
	"deepseek-r1-distill-llama-70b",
	"llama-3.3-70b-versatile",
	"llama-3.2-1b-preview",
	"llama-3.2-3b-preview",
	"llama-3.2-11b-vision-preview",
	"llama-3.2-90b-vision-preview",
	// "whisper-large-v3", // This is not a chat completion model
];

impl GroqAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "GROQ_API_KEY";
}

// The Groq API adapter is modeled after the OpenAI adapter, as the Groq API is compatible with the OpenAI API.
impl Adapter for GroqAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.groq.com/openai/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
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
		_service_target: crate::ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::Groq,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: crate::ModelIden,
		_web_response: crate::webc::WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(crate::Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::Groq,
			feature: "embeddings".to_string(),
		})
	}
}
