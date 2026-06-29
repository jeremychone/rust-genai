use super::macros::adapter_kind_str_maps;
use crate::adapter::Adapter as _;
use crate::adapter::adapters;
use crate::adapter::adapters::baidu::BAIDU_CODING_ANTHROPIC_NAMESPACE;
use crate::adapter::adapters::baidu::BAIDU_CODING_OPENAI_NAMESPACE;
use crate::adapter::adapters::zai;
use crate::{ModelName, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};

/// AdapterKind is an enum that represents the different types of adapters that can be used to interact with the API.
///
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AdapterKind {
	/// For OpenAI Chat Completions and also can be used for OpenAI compatible APIs
	/// NOTE: This adapter share some behavior that other adapters can use while still providing some variant
	OpenAI,

	/// For OpenAI Responses API
	OpenAIResp,

	/// Gemini adapter supports gemini native protocol. e.g., support thinking budget.
	Gemini,

	/// Anthopric native protocol as well
	Anthropic,

	/// For fireworks.ai, mostly OpenAI.
	Fireworks,

	/// Together AI (Mostly uses OpenAI-compatible protocol)
	Together,

	/// Reuse some of the OpenAI adapter behavior, customize some (e.g., normalize thinking budget)
	Groq,

	/// For AIHubMix (Mostly use OpenAI)
	Aihubmix,

	/// For kimi
	Kimi,

	/// For Mimo (Mostly use OpenAI)
	Mimo,

	/// For Moonshot AI (Mostly use OpenAI)
	Moonshot,

	/// For Nebius (Mostly use OpenAI)
	Nebius,

	/// For xAI (Mostly use OpenAI)
	Xai,

	/// For DeepSeek (Mostly use OpenAI)
	DeepSeek,

	/// For ZAI (Mostly use OpenAI)
	Zai,

	/// For big model (only accessible via namespace bigmodel::)
	BigModel,

	/// For aliyun (Mostly use OpenAI)
	Aliyun,

	/// For baidu (Mostly use OpenAI)
	Baidu,

	/// Cohere today use it's own native protocol but might move to OpenAI Adapter
	Cohere,

	/// OpenAI shared behavior + some custom. (currently, localhost only, can be customize with ServerTargetResolver).
	Ollama,

	/// For Ollama Cloud (ollama.com) - uses native Ollama protocol with Bearer auth
	OllamaCloud,

	/// Omlx adapter - OpenAI-compatible with reasoning model chat_template_kwargs injection
	Omlx,

	/// Google Vertex AI (Model Garden). Supports Gemini and Claude models via publishers/google and publishers/anthropic.
	/// Uses namespace routing: `vertex::gemini-2.5-flash`, `vertex::claude-sonnet-4-6`
	Vertex,

	/// GitHub Models inference API (multi-publisher gateway for OpenAI, Anthropic, and Google models).
	/// Uses namespace routing: `github_copilot::openai/gpt-4.1-mini`, `github_copilot::anthropic/claude-sonnet-4-6`, `github_copilot::google/gemini-2.5-pro`
	GithubCopilot,

	/// OpenCode Go proxy (OpenAI-compatible adapter for the OpenCode ecosystem).
	/// Namespace: `opencode_go::model-name` — route any model via the OpenCode Go gateway.
	OpenCodeGo,

	/// AWS Bedrock Converse API, authenticated with a simple Bearer token from
	/// `BEDROCK_API_KEY`. Always available — no extra Cargo feature or dependencies required.
	/// Namespace: `bedrock_api::anthropic.claude-sonnet-4-5-20250929-v1:0`.
	BedrockApi,

	/// AWS Bedrock Converse API, authenticated via SigV4 + the AWS credential chain
	/// (env, profile, SSO, IMDS, AssumeRole).
	/// Namespace: `bedrock_sigv4::anthropic.claude-sonnet-4-5-20250929-v1:0`.
	/// Requires the `bedrock-sigv4` Cargo feature.
	#[cfg(feature = "bedrock-sigv4")]
	BedrockSigv4,

	/// OpenRouter — OpenAI-compatible gateway for many providers (OpenAI, Anthropic, Google, etc.).
	/// Namespace: `open_router::openai/gpt-4.1`, `open_router::anthropic/claude-sonnet-4-5`.
	/// Uses `OPEN_ROUTER_API_KEY`.
	OpenRouter,

	/// For MiniMax (Anthropic-compatible protocol)
	MiniMax,

	/// Those are the Custom Adapter triggered by the `genai_` prefix namespaced
	/// e.g. `genai_1::gemma-4-26b-a4b-it-4bit` this will resolve endpoint, ... from env
	/// `GENAI_1_ENDPOINT`: required, e.g. `https://127.0.0.1:8000/v1`
	/// `GENAI_1_API_KEY`: optional, e.g. `welcome`
	/// For now, default to the "OpenAI" protocol, but will be able to set later.
	#[display("genai_{_0}")]
	Custom(u8),
}

// region:    --- str & default_key_env_name impl (via macro)

// The single source of truth for the string maps: one row per variant.
adapter_kind_str_maps! {
	OpenAI        => "OpenAI",        "openai",         adapters::all_adapters::OpenAIAdapter;
	OpenAIResp    => "OpenAIResp",    "openai_resp",    adapters::all_adapters::OpenAIRespAdapter;
	Gemini        => "Gemini",        "gemini",         adapters::all_adapters::GeminiAdapter;
	Anthropic     => "Anthropic",     "anthropic",      adapters::all_adapters::AnthropicAdapter;
	Fireworks     => "Fireworks",     "fireworks",      adapters::all_adapters::FireworksAdapter;
	Together      => "Together",      "together",       adapters::all_adapters::TogetherAdapter;
	Groq          => "Groq",          "groq",           adapters::all_adapters::GroqAdapter;
	Aihubmix      => "Aihubmix",      "aihubmix",       adapters::all_adapters::AihubmixAdapter;
	Kimi          => "Kimi",          "kimi",           adapters::all_adapters::KimiAdapter;
	Mimo          => "Mimo",          "mimo",           adapters::all_adapters::MimoAdapter;
	Moonshot      => "Moonshot",      "moonshot",       adapters::all_adapters::MoonshotAdapter;
	Nebius        => "Nebius",        "nebius",         adapters::all_adapters::NebiusAdapter;
	Xai           => "Xai",           "xai",            adapters::all_adapters::XaiAdapter;
	DeepSeek      => "DeepSeek",      "deepseek",       adapters::all_adapters::DeepSeekAdapter;
	Zai           => "Zai",           "zai",            adapters::all_adapters::ZaiAdapter;
	BigModel      => "BigModel",      "bigmodel",       adapters::all_adapters::BigModelAdapter;
	Aliyun        => "Aliyun",        "aliyun",         adapters::all_adapters::AliyunAdapter;
	Baidu         => "Baidu",         "baidu",          adapters::all_adapters::BaiduAdapter;
	Cohere        => "Cohere",        "cohere",         adapters::all_adapters::CohereAdapter;
	Ollama        => "Ollama",        "ollama",         adapters::all_adapters::OllamaAdapter;
	OllamaCloud   => "OllamaCloud",   "ollama_cloud",   adapters::all_adapters::OllamaCloudAdapter;
	Omlx          => "Omlx",          "omlx",           adapters::all_adapters::OmlxAdapter;
	Vertex        => "Vertex",        "vertex",         adapters::all_adapters::VertexAdapter;
	GithubCopilot => "GithubCopilot", "github_copilot", adapters::all_adapters::GithubCopilotAdapter;
	OpenCodeGo    => "OpenCodeGo",    "opencode_go",    adapters::all_adapters::OpenCodeGoAdapter;
	BedrockApi    => "BedrockApi",    "bedrock_api",    adapters::all_adapters::BedrockApiAdapter;
	OpenRouter    => "OpenRouter",    "open_router",    adapters::all_adapters::OpenRouterAdapter;
	MiniMax       => "MiniMax",       "minimax",        adapters::all_adapters::MiniMaxAdapter;
}

// endregion: --- str & default_key_env_name impl (via macro)

/// From Model implementations
impl AdapterKind {
	/// This is a default static mapping from model names to AdapterKind.
	///
	/// When more control is needed, the `ServiceTypeResolver` can be used
	/// to map a model name to any adapter and endpoint.
	///
	///  - OpenAI     - starts_with "gpt", "o3", "o1", "chatgpt"
	///  - Gemini     - starts_with "gemini"
	///  - Anthropic  - starts_with "claude"
	///  - Fireworks  - contains "fireworks" (might add leading or trailing '/' later)
	///  - Groq       - model in Groq models
	///  - DeepSeek   - model in DeepSeek models (deepseek.com)
	///  - Zhipu      - starts_with "glm"
	///  - Cohere     - starts_with "command"
	///  - Ollama     - For anything else
	///
	/// Other Some adapters have to have model name namespaced to be used,
	/// - e.g., for together.ai `together::meta-llama/Llama-3-8b-chat-hf`
	/// - e.g., for nebius with `nebius::Qwen/Qwen3-235B-A22B`
	/// - e.g., for ZAI coding plan with `zai_coding::glm-4.6`
	/// - e.g., for vertex with `vertex::gemini-2.5-flash` or `vertex::claude-sonnet-4-6`
	///
	/// And all adapters can be force namspaced as well.
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	///
	/// IMPORTANT: Since v0.6.0, Groq and Deepseek models needs to be namespaced e.g., `groq::_model_name_`
	//             (because now, list_names are dynamic, so, automatic mapping can only be done base on clear model "prefixes")
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let Some(adapter) = Self::from_model_namespace(model) {
			return Ok(adapter);
		};

		// -- Otherwise, Resolve from modelname
		if model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
			|| model.starts_with("codex")
			|| (model.starts_with("gpt") && !model.starts_with("gpt-oss"))
			|| model.starts_with("text-embedding")
		// migh be a little generic on this one
		{
			if model.starts_with("gpt-5")
				|| (model.starts_with("gpt") && (model.contains("codex") || model.contains("pro")))
			{
				Ok(Self::OpenAIResp)
			} else {
				Ok(Self::OpenAI)
			}
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.contains("fireworks") {
			Ok(Self::Fireworks)
		} else if model.starts_with("kimi-") {
			Ok(Self::Kimi)
		} else if model.starts_with("mimo-") {
			Ok(Self::Mimo)
		} else if model.starts_with("command") || model.starts_with("embed-") {
			Ok(Self::Cohere)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if model.starts_with("glm") {
			Ok(Self::Zai)
		} else if model.starts_with("deepseek-") {
			Ok(Self::DeepSeek)
		} else if model.starts_with("moonshot-") {
			Ok(Self::Moonshot)
		} else if model.starts_with("MiniMax-") || model.starts_with("minimax-") {
			Ok(Self::MiniMax)
		} else if model.starts_with("omlx-") {
			Ok(Self::Omlx)
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

/// Inner api to return an adatper type from an eventual namespaced model (e.g., `zai::glm-5.2`)
impl AdapterKind {
	pub(crate) fn from_model_namespace(model: &str) -> Option<Self> {
		let (namespace, _) = ModelName::split_as_namespace_and_name(model);
		let namespace = namespace?;

		// -- First, check if simple adapter lower string match
		if let Some(adapter) = Self::from_lower_str(namespace) {
			Some(adapter)
		}
		// -- Second, custom namespaces
		else if namespace == zai::ZAI_CODING_NAMESPACE {
			Some(Self::Zai)
		} else if namespace == BAIDU_CODING_OPENAI_NAMESPACE || namespace == BAIDU_CODING_ANTHROPIC_NAMESPACE {
			Some(Self::Baidu)
		}
		//
		// -- Otherwise, no adapter from namespace, because no matching namespace
		else {
			None
		}
	}
}
