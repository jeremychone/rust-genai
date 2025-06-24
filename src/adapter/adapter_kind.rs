use crate::adapter::anthropic::AnthropicAdapter;
use crate::adapter::cohere::CohereAdapter;
use crate::adapter::deepseek::{self, DeepSeekAdapter};
use crate::adapter::gemini::GeminiAdapter;
use crate::adapter::groq::{self, GroqAdapter};
use crate::adapter::nebius::NebiusAdapter;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::xai::XaiAdapter;
use crate::{ModelName, Result};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use tracing::info;

/// AdapterKind is an enum that represents the different types of adapters that can be used to interact with the API.
#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AdapterKind {
	/// Main adapter type for the OpenAI service.
	OpenAI,
	/// Used for the Ollama adapter (currently, localhost only). Behind the scenes, it uses the OpenAI adapter logic.
	Ollama,
	/// Used for the Anthropic adapter.
	Anthropic,
	/// Used for the Cohere adapter.
	Cohere,
	/// Used for the Gemini adapter.
	Gemini,
	/// Used for the Groq adapter. Behind the scenes, it uses the OpenAI adapter logic with the necessary Groq differences (e.g., usage).
	Groq,
	/// For Nebius
	Nebius,
	/// For xAI
	Xai,
	/// For DeepSeek
	DeepSeek,
	// Note: Variants will probably be suffixed
	// AnthropicBedrock,
}

/// Serialization/Parse implementations
impl AdapterKind {
	/// Serialize to a static str
	pub fn as_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "OpenAI",
			AdapterKind::Ollama => "Ollama",
			AdapterKind::Anthropic => "Anthropic",
			AdapterKind::Cohere => "Cohere",
			AdapterKind::Gemini => "Gemini",
			AdapterKind::Groq => "Groq",
			AdapterKind::Nebius => "Nebius",
			AdapterKind::Xai => "xAi",
			AdapterKind::DeepSeek => "DeepSeek",
		}
	}

	/// Serialize to a static str
	pub fn as_lower_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "openai",
			AdapterKind::Ollama => "ollama",
			AdapterKind::Anthropic => "anthropic",
			AdapterKind::Cohere => "cohere",
			AdapterKind::Gemini => "gemini",
			AdapterKind::Groq => "groq",
			AdapterKind::Nebius => "nebius",
			AdapterKind::Xai => "xai",
			AdapterKind::DeepSeek => "deepseek",
		}
	}

	pub fn from_lower_str(name: &str) -> Option<Self> {
		match name {
			"openai" => Some(AdapterKind::OpenAI),
			"ollama" => Some(AdapterKind::Ollama),
			"anthropic" => Some(AdapterKind::Anthropic),
			"cohere" => Some(AdapterKind::Cohere),
			"gemini" => Some(AdapterKind::Gemini),
			"groq" => Some(AdapterKind::Groq),
			"nebius" => Some(AdapterKind::Nebius),
			"xai" => Some(AdapterKind::Xai),
			"deepseek" => Some(AdapterKind::DeepSeek),
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
			AdapterKind::Anthropic => Some(AnthropicAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Cohere => Some(CohereAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Gemini => Some(GeminiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Groq => Some(GroqAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Nebius => Some(NebiusAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::Xai => Some(XaiAdapter::API_KEY_DEFAULT_ENV_NAME),
			AdapterKind::DeepSeek => Some(DeepSeekAdapter::API_KEY_DEFAULT_ENV_NAME),
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
	///  - Anthropic  - starts_with "claude"
	///  - Cohere     - starts_with "command"
	///  - Gemini     - starts_with "gemini"
	///  - Groq       - model in Groq models
	///  - DeepSeek   - model in DeepSeek models (deepseek.com)
	///  - Ollama     - For anything else
	///
	/// Note: At this point, this will never fail as the fallback is the Ollama adapter.
	///       This might change in the future, hence the Result return type.
	pub fn from_model(model: &str) -> Result<Self> {
		// -- First check if namespaced
		if let (_, Some(ns)) = ModelName::model_name_and_namespace(model) {
			if let Some(adapter) = Self::from_lower_str(ns) {
				return Ok(adapter);
			} else {
				// TODO: Reassess if we need to returns error if no matching adapters
				info!("No AdapterKind found for '{ns}'")
			}
		}

		// -- Resolve from modelname
		if model.starts_with("gpt")
			|| model.starts_with("o3")
			|| model.starts_with("o4")
			|| model.starts_with("o1")
			|| model.starts_with("chatgpt")
		{
			Ok(Self::OpenAI)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.starts_with("command") {
			Ok(Self::Cohere)
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if model.starts_with("grok") {
			Ok(Self::Xai)
		} else if deepseek::MODELS.contains(&model) {
			Ok(Self::DeepSeek)
		} else if groq::MODELS.contains(&model) {
			return Ok(Self::Groq);
		}
		// For now, fallback to Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}
