use crate::ModelIden;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct NebiusAdapter;

// Latest models
// NOTE: These are only used for the list_names API (not for Nebius model matching).
// Use `nebius::deepseek-ai/DeepSeek-R1-0528` as the model name to select the Nebius adapter.
pub(in crate::adapter) const MODELS: &[&str] = &[
	//
	"deepseek-ai/DeepSeek-R1-0528",
	"Qwen/Qwen3-235B-A22B",
	"Qwen/Qwen3-30B-A3B",
	"Qwen/Qwen3-32B",
	"Qwen/Qwen3-14B",
	"Qwen/Qwen3-4B-fast",
	"nvidia/Llama-3_1-Nemotron-Ultra-253B-v1",
	"deepseek-ai/DeepSeek-V3-0324",
	"deepseek-ai/DeepSeek-V3",
	"deepseek-ai/DeepSeek-R1",
	"meta-llama/Llama-3.3-70B-Instruct",
	"meta-llama/Meta-Llama-3.1-70B-Instruct",
	"meta-llama/Meta-Llama-3.1-8B-Instruct",
	"meta-llama/Meta-Llama-3.1-405B-Instruct",
	"mistralai/Mistral-Nemo-Instruct-2407",
	"Qwen/Qwen2.5-Coder-7B",
	"Qwen/Qwen2.5-Coder-32B-Instruct",
	"google/gemma-2-2b-it",
	"google/gemma-2-9b-it-fast",
	"Qwen/Qwen2.5-32B-Instruct",
	"Qwen/Qwen2.5-72B-Instruct",
	"aaditya/Llama3-OpenBioLLM-70B",
	"Qwen/QwQ-32B",
	"microsoft/phi-4",
	"NousResearch/Hermes-3-Llama-405B",
	"deepseek-ai/DeepSeek-R1-Distill-Llama-70B",
	"nvidia/Llama-3_3-Nemotron-Super-49B-v1",
];

impl NebiusAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "NEBIUS_API_KEY";
}

// The Nebius API adapter is modeled after the OpenAI adapter, as the Nebius API is compatible with the OpenAI API.
impl Adapter for NebiusAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://api.studio.nebius.ai/v1/";
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
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}
