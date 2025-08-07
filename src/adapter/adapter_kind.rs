use crate::adapter::adapters::together::TogetherAdapter;
use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::deepseek::{self, DeepSeekAdapter};
use crate::adapter::fireworks::FireworksAdapter;
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::groq::{self, GroqAdapter};
use crate::adapter::nebius::NebiusAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::adapter::zhipu::ZhipuAdapter;
use crate::{ModelName, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use tracing::info;

/// AdapterKind is an enum that represents the different types of adapters that can be used to interact with the API.
///
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AdapterKind {
	/// For OpenAI and also can be used for OpenAI compatible APIs
	/// NOTE: This adapter share some behavior that other adapters can use while still providing some variant
	OpenAI,
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
	/// For Nebius (Mostly use OpenAI)
	Nebius,
	/// For xAI (Mostly use OpenAI)
	Xai,
	/// For DeepSeek (Mostly use OpenAI)
	DeepSeek,
	/// For Zhipu (Mostly use OpenAI)
	Zhipu,
	/// Cohere today use it's own native protocol but might move to OpenAI Adapter
	Cohere,
	/// OpenAI shared behavior + some custom. (currently, localhost only, can be customize with ServerTargetResolver).
	Ollama,
}

/// Serialization/Parse implementations
impl AdapterKind {
	/// Serialize to a static str
	pub fn as_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "OpenAI",
			AdapterKind::Gemini => "Gemini",
			AdapterKind::Anthropic => "Anthropic",
			AdapterKind::Fireworks => "Fireworks",
			AdapterKind::Together => "Together",
			AdapterKind::Groq => "Groq",
			AdapterKind::Nebius => "Nebius",
			AdapterKind::Xai => "xAi",
			AdapterKind::DeepSeek => "DeepSeek",
			AdapterKind::Zhipu => "Zhipu",
			AdapterKind::Cohere => "Cohere",
			AdapterKind::Ollama => "Ollama",
		}
	}

	/// Serialize to a lowercase static str
	pub fn as_lower_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "openai",
			AdapterKind::Gemini => "gemini",
			AdapterKind::Anthropic => "anthropic",
			AdapterKind::Fireworks => "fireworks",
			AdapterKind::Together => "together",
			AdapterKind::Groq => "groq",
			AdapterKind::Nebius => "nebius",
			AdapterKind::Xai => "xai",
			AdapterKind::DeepSeek => "deepseek",
			AdapterKind::Zhipu => "zhipu",
			AdapterKind::Cohere => "cohere",
			AdapterKind::Ollama => "ollama",
		}
	}

	pub fn from_lower_str(name: &str) -> Option<Self> {
		match name {
			"openai" => Some(AdapterKind::OpenAI),
			"gemini" => Some(AdapterKind::Gemini),
			"anthropic" => Some(AdapterKind::Anthropic),
			"fireworks" => Some(AdapterKind::Fireworks),
			"together" => Some(AdapterKind::Together),
			"groq" => Some(AdapterKind::Groq),
			"nebius" => Some(AdapterKind::Nebius),
			"xai" => Some(AdapterKind::Xai),
			"deepseek" => Some(AdapterKind::DeepSeek),
			"zhipu" => Some(AdapterKind::Zhipu),
			"cohere" => Some(AdapterKind::Cohere),
			"ollama" => Some(AdapterKind::Ollama),
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
			AdapterKind::Gemini => Some(GeminiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Anthropic => Some(AnthropicAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Fireworks => Some(FireworksAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Together => Some(TogetherAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Groq => Some(GroqAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Nebius => Some(NebiusAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Xai => Some(XaiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::DeepSeek => Some(DeepSeekAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Zhipu => Some(ZhipuAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Cohere => Some(CohereAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Ollama => None,
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
	///  - Zhipu      - starts_with "glm"
	///  - Cohere     - starts_with "command"
	///  - Ollama     - For anything else
	///
	/// Other Some adapters have to have model name namespaced to be used,
	/// - e.g., for together.ai `together::meta-llama/Llama-3-8b-chat-hf`
	/// - e.g., for nebius with `nebius::Qwen/Qwen3-235B-A22B`
	///
	/// And all adapters can be force namspaced as well.
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let (_, Some(ns)) = ModelName::model_name_and_namespace(model) {
			if let Some(adapter) = Self::from_lower_str(ns) {
				return Ok(adapter);
			} else {
				info!("No AdapterKind found for '{ns}'")
			}
		}

		// -- Resolve from modelname
		if model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
			|| model.starts_with("codex")
			|| (model.starts_with("gpt") && !model.starts_with("gpt-oss"))
			|| model.starts_with("text-embedding")
		// migh be a little generic on this one
		{
			Ok(Self::OpenAI)
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.contains("fireworks") {
			Ok(Self::Fireworks)
		} else if groq::MODELS.contains(&model) {
			Ok(Self::Groq)
		} else if model.starts_with("command") || model.starts_with("embed-") {
			Ok(Self::Cohere)
		} else if deepseek::MODELS.contains(&model) {
			Ok(Self::DeepSeek)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if model.starts_with("glm") {
			Ok(Self::Zhipu)
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}
