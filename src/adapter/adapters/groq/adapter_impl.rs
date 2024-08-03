use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::Result;
use crate::{ClientConfig, ModelInfo};
use reqwest::RequestBuilder;

pub struct GroqAdapter;

const BASE_URL: &str = "https://api.groq.com/openai/v1/";
pub(in crate::adapter) const MODELS: &[&str] = &[
	"llama-3.1-405b-reasoning",
	"llama-3.1-70b-versatile",
	"llama-3.1-8b-instant",
	"mixtral-8x7b-32768",
	"gemma-7b-it",
	"gemma2-9b-it",
	"llama3-groq-70b-8192-tool-use-preview",
	"llama3-groq-8b-8192-tool-use-preview",
	"llama3-8b-8192",
	"llama3-70b-8192",
	// "whisper-large-v3", // This is not a chat completion model
];

// The Groq API adapter is modeled after the OpenAI adapter, as the Groq API is compatible with the OpenAI API.
impl Adapter for GroqAdapter {
	fn default_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		Some("GROQ_API_KEY")
	}

	async fn all_model_names(_kind: AdapterKind) -> Result<Vec<String>> {
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	fn get_service_url(model_info: ModelInfo, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(model_info, service_type, BASE_URL)
	}

	fn to_web_request_data(
		model_info: ModelInfo,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let url = Self::get_service_url(model_info.clone(), service_type);

		OpenAIAdapter::util_to_web_request_data(model_info, client_config, chat_req, service_type, options_set, url)
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
