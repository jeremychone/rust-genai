use super::groq::MODELS as GROQ_MODELS;
use crate::Result;
use derive_more::Display;

#[derive(Debug, Clone, Copy, Display, Eq, PartialEq, Hash)]
pub enum AdapterKind {
	OpenAI,
	Ollama,
	Anthropic,
	Cohere,
	Gemini,
	Groq,
	// Note: Variants will probalby be suffixed
	// AnthropicBerock,
}

/// Serialization impls
impl AdapterKind {
	pub fn as_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "OpenAI",
			AdapterKind::Ollama => "Ollama",
			AdapterKind::Anthropic => "Anthropic",
			AdapterKind::Cohere => "Cohere",
			AdapterKind::Gemini => "Gemini",
			AdapterKind::Groq => "Groq",
		}
	}

	pub fn as_lower_str(&self) -> &'static str {
		match self {
			AdapterKind::OpenAI => "openai",
			AdapterKind::Ollama => "ollama",
			AdapterKind::Anthropic => "anthropic",
			AdapterKind::Cohere => "cohere",
			AdapterKind::Gemini => "gemini",
			AdapterKind::Groq => "groq",
		}
	}
}

/// From Model impls
impl AdapterKind {
	/// Very simplistic mapper for now.
	///  - starts_with "gpt"      -> OpenAI
	///  - starts_with "claude"   -> Anthropic
	///  - starts_with "command"  -> Cohere
	///  - starts_with "gemini"   -> Gemini
	///  - model in Groq models   -> Groq
	///  - For anything else      -> Ollama
	pub fn from_model(model: &str) -> Result<Self> {
		if model.starts_with("gpt") {
			Ok(Self::OpenAI)
		} else if model.starts_with("claude") {
			Ok(Self::Anthropic)
		} else if model.starts_with("command") {
			Ok(Self::Cohere)
		} else if model.starts_with("gemini") {
			Ok(Self::Gemini)
		} else if GROQ_MODELS.contains(&model) {
			return Ok(Self::Groq);
		}
		// for now, fallback on Ollama
		else {
			Ok(Self::Ollama)
		}
	}
}
