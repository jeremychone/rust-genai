use std::sync::OnceLock;

use reqwest::RequestBuilder;

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::support::get_api_key_resolver;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatRequestOptionsSet, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ConfigSet, Result};

pub struct GroqAdapter;

const BASE_URL: &str = "https://api.groq.com/openai/v1/";
pub(in crate::adapter) const MODELS: &[&str] = &[
	"llama3-8b-8192",
	"llama3-70b-8192",
	"mixtral-8x7b-32768",
	"gemma-7b-it",
	"whisper-large-v3",
];

// The Groq API adapter is modeled after the OpenAI adapter, as the Groq API is compatible with the OpenAI API.
impl Adapter for GroqAdapter {
	async fn list_models(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(|| AdapterConfig::default().with_auth_env_name("GROQ_API_KEY"))
	}

	fn get_service_url(kind: AdapterKind, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(kind, service_type, BASE_URL)
	}

	fn to_web_request_data(
		kind: AdapterKind,
		config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		model: &str,
		chat_req: ChatRequest,
		options_set: ChatRequestOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let api_key = get_api_key_resolver(kind, config_set)?;
		let url = Self::get_service_url(kind, service_type);

		OpenAIAdapter::util_to_web_request_data(kind, url, model, chat_req, service_type, options_set, &api_key, false)
	}

	fn to_chat_response(kind: AdapterKind, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(kind, web_response)
	}

	fn to_chat_stream(kind: AdapterKind, reqwest_builder: RequestBuilder) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(kind, reqwest_builder)
	}
}
