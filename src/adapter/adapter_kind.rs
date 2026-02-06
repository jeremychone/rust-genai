use crate::adapter::adapters::together::TogetherAdapter;
use crate::adapter::adapters::zai::ZaiAdapter;
use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::bedrock::{self, BedrockAdapter};
use crate::adapter::bigmodel::BigModelAdapter;
use crate::adapter::cerebras::CerebrasAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::deepseek::{self, DeepSeekAdapter};
use crate::adapter::fireworks::FireworksAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::groq::{self, GroqAdapter};
use crate::adapter::mimo::{self, MimoAdapter};
use crate::adapter::nebius::NebiusAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::openrouter::OpenRouterAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::adapter::zai;
use crate::adapter::zhipu::ZhipuAdapter;
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
	/// For OpenRouter (OpenAI-compatible protocol)
	OpenRouter,
	/// Anthopric native protocol as well
	Anthropic,
	/// For fireworks.ai, mostly OpenAI.
	Fireworks,
	/// Together AI (Mostly uses OpenAI-compatible protocol)
	Together,
	/// Reuse some of the OpenAI adapter behavior, customize some (e.g., normalize thinking budget)
	Groq,
	/// For Mimo (Mostly use OpenAI)
	Mimo,
	/// For Nebius (Mostly use OpenAI)
	Nebius,
	/// For xAI (Mostly use OpenAI)
	Xai,
	/// For DeepSeek (Mostly use OpenAI)
	DeepSeek,
	/// For ZAI (OpenAI-compatible with dual endpoint support: zai:: and zai-coding::)
	Zai,
	/// For big model (only accessible via namespace bigmodel::)
	BigModel,
	/// Cohere today use it's own native protocol but might move to OpenAI Adapter
	Cohere,
	/// OpenAI shared behavior + some custom. (currently, localhost only, can be customize with ServerTargetResolver).
	Ollama,
	/// Cerebras (OpenAI-compatible protocol)
	Cerebras,
	/// AWS Bedrock (uses Converse API with Bearer token authentication)
	Bedrock,
	/// For Zhipu (legacy, kept for backwards compatibility)
	Zhipu,
}

/// Serialization/Parse implementations
impl AdapterKind {
	/// Serialize to a static str
	pub fn as_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "OpenAI",
			AdapterKind::OpenAIResp => "OpenAIResp",
			AdapterKind::Gemini => "Gemini",
			AdapterKind::Anthropic => "Anthropic",
			AdapterKind::OpenRouter => "OpenRouter",
			AdapterKind::Fireworks => "Fireworks",
			AdapterKind::Together => "Together",
			AdapterKind::Groq => "Groq",
			AdapterKind::Mimo => "Mimo",
			AdapterKind::Nebius => "Nebius",
			AdapterKind::Xai => "xAi",
			AdapterKind::DeepSeek => "DeepSeek",
			AdapterKind::Zai => "Zai",
			AdapterKind::BigModel => "BigModel",
			AdapterKind::Cohere => "Cohere",
			AdapterKind::Ollama => "Ollama",
			AdapterKind::Cerebras => "Cerebras",
			AdapterKind::Bedrock => "Bedrock",
			AdapterKind::Zhipu => "Zhipu",
		}
	}

	/// Serialize to a lowercase static str
	pub fn as_lower_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "openai",
			AdapterKind::OpenAIResp => "openai_resp",
			AdapterKind::Gemini => "gemini",
			AdapterKind::Anthropic => "anthropic",
			AdapterKind::OpenRouter => "openrouter",
			AdapterKind::Fireworks => "fireworks",
			AdapterKind::Together => "together",
			AdapterKind::Groq => "groq",
			AdapterKind::Mimo => "mimo",
			AdapterKind::Nebius => "nebius",
			AdapterKind::Xai => "xai",
			AdapterKind::DeepSeek => "deepseek",
			AdapterKind::Zai => "zai",
			AdapterKind::BigModel => "bigmodel",
			AdapterKind::Cohere => "cohere",
			AdapterKind::Ollama => "ollama",
			AdapterKind::Cerebras => "cerebras",
			AdapterKind::Bedrock => "bedrock",
			AdapterKind::Zhipu => "zhipu",
		}
	}

	pub fn from_lower_str(name: &str) -> Option<Self> {
		match name {
			"openai" => Some(AdapterKind::OpenAI),
			"openai_resp" => Some(AdapterKind::OpenAIResp),
			"gemini" => Some(AdapterKind::Gemini),
			"anthropic" => Some(AdapterKind::Anthropic),
			"openrouter" => Some(AdapterKind::OpenRouter),
			"fireworks" => Some(AdapterKind::Fireworks),
			"together" => Some(AdapterKind::Together),
			"groq" => Some(AdapterKind::Groq),
			"mimo" => Some(AdapterKind::Mimo),
			"nebius" => Some(AdapterKind::Nebius),
			"xai" => Some(AdapterKind::Xai),
			"deepseek" => Some(AdapterKind::DeepSeek),
			"zai" => Some(AdapterKind::Zai),
			"bigmodel" => Some(AdapterKind::BigModel),
			"cohere" => Some(AdapterKind::Cohere),
			"ollama" => Some(AdapterKind::Ollama),
			"cerebras" => Some(AdapterKind::Cerebras),
			"bedrock" => Some(AdapterKind::Bedrock),
			"zhipu" => Some(AdapterKind::Zhipu),
			_ => None,
		}
	}
}

/// Utilities
impl AdapterKind {
	/// Get the default key environment variable name for the adapter kind.
	pub fn default_key_env_name(&self) -> Option<&'static str> {
		match self {
			AdapterKind::OpenAI => Some(OpenAIAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::OpenAIResp => Some(OpenAIAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Gemini => Some(GeminiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Anthropic => Some(AnthropicAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::OpenRouter => Some(OpenRouterAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Fireworks => Some(FireworksAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Together => Some(TogetherAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Groq => Some(GroqAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Mimo => Some(MimoAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Nebius => Some(NebiusAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Xai => Some(XaiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::DeepSeek => Some(DeepSeekAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Zai => Some(ZaiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::BigModel => Some(BigModelAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Cohere => Some(CohereAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Ollama => None,
			AdapterKind::Cerebras => Some(CerebrasAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Bedrock => Some(BedrockAdapter::API_KEY_ENV),
			AdapterKind::Zhipu => Some(ZhipuAdapter::API_KEY_DEFAULT_ENV_NAME),
		}
	}
}

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
	///  - Zai        - model in ZAI models (glm series)
	///  - Cohere     - starts_with "command"
	///  - Ollama     - For anything else
	///
	/// Other Some adapters have to have model name namespaced to be used,
	/// - e.g., for together.ai `together::meta-llama/Llama-3-8b-chat-hf`
	/// - e.g., for nebius with `nebius::Qwen/Qwen3-235B-A22B`
	/// - e.g., for cerebras with `cerebras::llama-3.1-8b`
	/// - e.g., for ZAI coding plan with `zai-coding::glm-4.6`
	///
	/// And all adapters can be force namspaced as well.
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let Some(adapter) = Self::from_model_namespace(model) {
			return Ok(adapter);
		};

		// -- Special handling for OpenRouter models (they start with provider names)
		//    Only catch patterns without explicit :: namespace
		if model.contains('/')
			&& !model.contains("::")
			&& (model.starts_with("openai/")
				|| model.starts_with("anthropic/")
				|| model.starts_with("meta-llama/")
				|| model.starts_with("google/"))
		{
			return Ok(Self::OpenRouter);
		}

		// -- Otherwise, Resolve from modelname
		if model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
			|| model.starts_with("codex")
			|| (model.starts_with("gpt") && !model.starts_with("gpt-oss"))
			|| model.starts_with("text-embedding")
		{
			if model.starts_with("gpt") && (model.contains("codex") || model.contains("pro")) {
				Ok(Self::OpenAIResp)
			} else {
				Ok(Self::OpenAI)
			}
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if zai::MODELS.contains(&model) {
			Ok(Self::Zai)
		} else if model.contains("fireworks") {
			Ok(Self::Fireworks)
		} else if groq::MODELS.contains(&model) {
			Ok(Self::Groq)
		} else if mimo::MODELS.contains(&model) {
			Ok(Self::Mimo)
		} else if model.starts_with("command") || model.starts_with("embed-") {
			Ok(Self::Cohere)
		} else if deepseek::MODELS.contains(&model) {
			Ok(Self::DeepSeek)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if model.starts_with("glm") {
			Ok(Self::Zai)
		}
		// AWS Bedrock models (provider.model-name format)
		else if bedrock::MODELS.contains(&model) {
			Ok(Self::Bedrock)
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}

// region:    --- Support

/// Inner api to return
impl AdapterKind {
	fn from_model_namespace(model: &str) -> Option<Self> {
		let (namespace, _) = ModelName::split_as_namespace_and_name(model);
		let namespace = namespace?;

		// -- First, check if simple adapter lower string match
		if let Some(adapter) = Self::from_lower_str(namespace) {
			Some(adapter)
		}
		// -- Second, custom, for now, we hardcode this exception here (might become more generic later)
		else if namespace == zai::ZAI_CODING_NAMESPACE {
			Some(Self::Zai)
		}
		//
		// -- Otherwise, no adapter from namespace, because no matching namespace
		else {
			None
		}
	}
}

// endregion: --- Support
