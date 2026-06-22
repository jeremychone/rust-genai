// region:    --- BaiduAdapter

use crate::adapter::adapters::anthropic::AnthropicAdapter;
use crate::adapter::adapters::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebClient, WebResponse};
use crate::{ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

/// Baidu Adapter - Supports OpenAI-compatible and Anthropic-compatible APIs for Baidu Qianfan (Wenxin Workshop)
///
/// Baidu Qianfan API provides multiple protocols:
/// - OpenAI-compatible endpoints for chat, streaming, and embedding
/// - Anthropic-compatible endpoints for coding plan
///
/// This adapter handles routing to appropriate endpoints based on model namespace.
#[derive(Debug)]
pub struct BaiduAdapter;

/// Namespace for Baidu coding plan models using OpenAI protocol
pub const BAIDU_CODING_OPENAI_NAMESPACE: &str = "baidu-coding-openai";

/// Namespace for Baidu coding plan models using Anthropic protocol
pub const BAIDU_CODING_ANTHROPIC_NAMESPACE: &str = "baidu-coding-anthropic";

/// Helper structure to hold Baidu model parsing information
struct BaiduModelEndpoint {
	endpoint: Endpoint,
	protocol: BaiduProtocol,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BaiduProtocol {
	OpenAI,
	Anthropic,
}

impl BaiduModelEndpoint {
	/// Parse ModelIden to determine if it's a coding model and return endpoint with protocol
	fn from_model(model: &ModelIden) -> Self {
		let (namespace, _) = model.model_name.namespace_and_name();

		// Check namespace to determine endpoint and protocol
		let (endpoint, protocol) = match namespace {
			Some(BAIDU_CODING_OPENAI_NAMESPACE) => (
				Endpoint::from_static("https://qianfan.baidubce.com/v2/coding/"),
				BaiduProtocol::OpenAI,
			),
			Some(BAIDU_CODING_ANTHROPIC_NAMESPACE) => (
				Endpoint::from_static("https://qianfan.baidubce.com/anthropic/coding/"),
				BaiduProtocol::Anthropic,
			),
			_ => (
				BaiduAdapter::default_endpoint(AdapterKind::Baidu),
				BaiduProtocol::OpenAI,
			),
		};

		Self { endpoint, protocol }
	}
}

// All Baidu Qianfan supported models
// NOTE: These are sourced from the official Baidu Qianfan documentation
pub const MODELS: &[&str] = &[
	// ERNIE models (Chat) - Baidu's proprietary models
	"ERNIE-4.0-8K",
	"ERNIE-4.0-8K-Latest",
	"ERNIE-4.0-128K",
	"ERNIE-4.0-128K-Latest",
	"ERNIE-3.5-8K",
	"ERNIE-3.5-8K-Latest",
	"ERNIE-3.5-128K",
	"ERNIE-3.5-128K-Latest",
	"ERNIE-Lite-8K",
	"ERNIE-Lite-8K-Latest",
	"ERNIE-Speed-8K",
	"ERNIE-Speed-8K-Latest",
	"ERNIE-Speed-128K",
	"ERNIE-Speed-128K-Latest",
	"ERNIE-Speed-128K-0618",
	"ERNIE-Tiny-8K",
	"ERNIE-Tiny-8K-Latest",
	"ERNIE-4.0-8K-Preview",
	"ERNIE-4.0-8K-Preview-1228",
	"ERNIE-4.0-128K-Preview",
	"ERNIE-4.0-128K-Preview-1228",
	"ERNIE-4.0-8K-0325",
	"ERNIE-4.0-128K-0325",
	"ERNIE-3.5-8K-1222",
	"ERNIE-3.5-8K-0320",
	"ERNIE-3.5-128K-0320",
	// GLM models (ChatGLM series from Zhipu AI)
	"GLM-5.1",
	"GLM-5",
	"GLM-4-Flash",
	"GLM-4-Flash-Latest",
	"GLM-4-Plus",
	"GLM-4-Plus-Latest",
	"GLM-4-128K",
	"GLM-4-128K-Latest",
	"GLM-4-0520",
	"GLM-4-0520-Latest",
	"GLM-4-Air",
	"GLM-4-Air-Latest",
	"GLM-4-AirX",
	"GLM-4-AirX-Latest",
	"GLM-4-9B-Chat",
	"GLM-4-9B-Chat-Latest",
	"GLM-4-9B-Chat-1M",
	"GLM-4-9B-Chat-1M-Latest",
	// MiniMax models
	"MiniMax-Abba-5",
	"MiniMax-Abba-5-Latest",
	"MiniMax-Abba-5.5",
	"MiniMax-Abba-5.5-Latest",
	"MiniMax-Abba-5.5-Flash",
	"MiniMax-Abba-5.5-Flash-Latest",
	"MiniMax-Text-01",
	"MiniMax-Text-01-Latest",
	// Llama models
	"Llama-3.1-8B-Instruct",
	"Llama-3.1-8B-Instruct-Latest",
	"Llama-3.1-70B-Instruct",
	"Llama-3.1-70B-Instruct-Latest",
	"Llama-3.2-1B-Instruct",
	"Llama-3.2-1B-Instruct-Latest",
	"Llama-3.2-3B-Instruct",
	"Llama-3.2-3B-Instruct-Latest",
	"Llama-3.2-11B-Vision-Instruct",
	"Llama-3.2-11B-Vision-Instruct-Latest",
	"Llama-3.2-90B-Vision-Instruct",
	"Llama-3.2-90B-Vision-Instruct-Latest",
	// Qwen models (from Alibaba)
	"Qwen3-0.6B",
	"Qwen3-1.7B",
	"Qwen3-14B",
	"Qwen2.5-7B-Instruct",
	"Qwen2.5-7B-Instruct-Latest",
	"Qwen2.5-14B-Instruct",
	"Qwen2.5-14B-Instruct-Latest",
	"Qwen2.5-32B-Instruct",
	"Qwen2.5-32B-Instruct-Latest",
	"Qwen2.5-72B-Instruct",
	"Qwen2.5-72B-Instruct-Latest",
	"Qwen2.5-Coder-7B-Instruct",
	"Qwen2.5-Coder-7B-Instruct-Latest",
	"Qwen2.5-Coder-32B-Instruct",
	"Qwen2.5-Coder-32B-Instruct-Latest",
	// DeepSeek models
	"DeepSeek-V3.2",
	"DeepSeek-V3",
	"DeepSeek-V3-Latest",
	"DeepSeek-R1",
	"DeepSeek-R1-Latest",
	"DeepSeek-Coder-V2",
	"DeepSeek-Coder-V2-Latest",
	"DeepSeek-Coder-V2-Lite",
	"DeepSeek-Coder-V2-Lite-Latest",
	// Code models (coding plan)
	"CodeLlama-7b-Instruct",
	"CodeLlama-7b-Instruct-Latest",
	"CodeLlama-13b-Instruct",
	"CodeLlama-13b-Instruct-Latest",
	"CodeLlama-34b-Instruct",
	"CodeLlama-34b-Instruct-Latest",
	"CodeLlama-70b-Instruct",
	"CodeLlama-70b-Instruct-Latest",
	"Qianfan-CodeLlama-34b-Instruct",
	"Qianfan-CodeLlama-34b-Instruct-Latest",
	"Qianfan-DeepSeek-Coder-V2",
	"Qianfan-DeepSeek-Coder-V2-Latest",
	"StarCoder2-7B",
	"StarCoder2-7B-Latest",
	"StarCoder2-15B",
	"StarCoder2-15B-Latest",
	// Embedding models
	"Embedding-V1",
	"bge-large-zh",
	"bge-large-en",
	"tao-8k",
	"m3e-base",
	"m3e-large",
	// Text2Image models
	"Stable-Diffusion-XL",
	"Stable-Diffusion-XL-Latest",
	"ERNIE-ViLG",
	"ERNIE-ViLG-Latest",
	"Stable-Diffusion-3.5",
	"Stable-Diffusion-3.5-Latest",
	// Speech models
	"ERNIE-Speech",
	"ERNIE-Speech-Latest",
	"TTS",
	"TTS-Latest",
];
impl BaiduAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "BAIDU_API_KEY";
}

impl Adapter for BaiduAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	/// Returns the default endpoint for Baidu Qianfan API
	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const BASE_URL: &str = "https://qianfan.baidubce.com/v2/";
		Endpoint::from_static(BASE_URL)
	}

	/// Returns authentication data with API key from environment variable
	fn default_auth(_kind: AdapterKind) -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => AuthData::from_env(env_name),
			None => AuthData::None,
		}
	}

	/// Returns all supported model names for Baidu
	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData, _web_client: &WebClient) -> Result<Vec<String>> {
		// For coding endpoints, we might want to return coding-specific models
		// For now, return all models
		Ok(MODELS.iter().map(|s| s.to_string()).collect())
	}

	/// Returns the service URL for the given model
	///
	/// Baidu Qianfan API supports multiple protocols:
	/// - OpenAI-compatible: uses /chat/completions and /embeddings endpoints
	/// - Anthropic-compatible: uses /messages endpoint for chat
	fn get_service_url(model: &ModelIden, service_type: ServiceType, _endpoint: Endpoint) -> Result<String> {
		// Determine protocol and endpoint based on model namespace
		let baidu_info = BaiduModelEndpoint::from_model(model);
		let base_url = baidu_info.endpoint.base_url();

		let url = match (baidu_info.protocol, service_type) {
			// OpenAI protocol endpoints
			(BaiduProtocol::OpenAI, ServiceType::Chat | ServiceType::ChatStream) => {
				format!("{base_url}chat/completions")
			}
			(BaiduProtocol::OpenAI, ServiceType::Embed) => format!("{base_url}embeddings"),

			// Anthropic protocol endpoints (coding plan)
			(BaiduProtocol::Anthropic, ServiceType::Chat | ServiceType::ChatStream) => format!("{base_url}messages"),
			(BaiduProtocol::Anthropic, ServiceType::Embed) => {
				return Err(crate::Error::AdapterNotSupported {
					adapter_kind: AdapterKind::Baidu,
					feature: "embedding with Anthropic protocol".to_string(),
				});
			}
		};
		Ok(url)
	}

	/// Converts chat request data to web request format
	///
	/// Delegates to appropriate adapter based on protocol.
	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let baidu_info = BaiduModelEndpoint::from_model(&target.model);

		match baidu_info.protocol {
			BaiduProtocol::OpenAI => {
				OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
			}
			BaiduProtocol::Anthropic => {
				AnthropicAdapter::to_web_request_data(target, service_type, chat_req, chat_options)
			}
		}
	}

	/// Converts response bytes to ChatResponse
	///
	/// Delegates to appropriate adapter based on protocol.
	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		let baidu_info = BaiduModelEndpoint::from_model(&model_iden);

		match baidu_info.protocol {
			BaiduProtocol::OpenAI => OpenAIAdapter::to_chat_response(model_iden, web_response, options_set),
			BaiduProtocol::Anthropic => AnthropicAdapter::to_chat_response(model_iden, web_response, options_set),
		}
	}

	/// Converts response stream to ChatStream
	///
	/// Delegates to appropriate adapter based on protocol.
	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let baidu_info = BaiduModelEndpoint::from_model(&model_iden);

		match baidu_info.protocol {
			BaiduProtocol::OpenAI => OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
			BaiduProtocol::Anthropic => AnthropicAdapter::to_chat_stream(model_iden, reqwest_builder, options_set),
		}
	}

	/// Converts embedding request data to web request format
	///
	/// Delegates to OpenAIAdapter utilities due to API compatibility.
	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	/// Converts response bytes to EmbedResponse
	///
	/// Delegates to OpenAIAdapter due to API compatibility.
	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

// endregion: --- BaiduAdapter
