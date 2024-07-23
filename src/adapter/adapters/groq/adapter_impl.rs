use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::Result;
use crate::{ConfigSet, ModelInfo};
use reqwest::RequestBuilder;
use std::sync::OnceLock;

pub struct GroqAdapter;

const BASE_URL: &str = "https://api.groq.com/openai/v1/";
pub(in crate::adapter) const MODELS: &[&str] = &[
	"llama3-8b-8192",
	"llama3-70b-8192",
	"mixtral-8x7b-32768",
	"gemma-7b-it",
	"gemma2-9b-it",
	// "whisper-large-v3", // This is not a chat completion model
];

// The Groq API adapter is modeled after the OpenAI adapter, as the Groq API is compatible with the OpenAI API.
impl Adapter for GroqAdapter {
	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(|| AdapterConfig::default().with_auth_env_name("GROQ_API_KEY"))
	}

	fn get_service_url(model_info: ModelInfo, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(model_info, service_type, BASE_URL)
	}

	fn to_web_request_data(
		model_info: ModelInfo,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let api_key = get_api_key_resolver(model_info.clone(), config_set)?;
		let url = Self::get_service_url(model_info.clone(), service_type);

		OpenAIAdapter::util_to_web_request_data(model_info, url, chat_req, service_type, options_set, &api_key, false)
	}

	fn to_chat_response(model_info: ModelInfo, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_info, web_response)
	}

	fn to_chat_stream(
		model_info: ModelInfo,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_info, reqwest_builder, options_set)
	}
}
