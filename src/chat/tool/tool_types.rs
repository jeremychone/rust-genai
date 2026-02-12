use derive_more::{Display, From};
use serde::{Deserialize, Serialize, Serializer};

use crate::chat::WebSearchConfig;

/// Normalized tool identifiers.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Display, From)]
pub enum ToolName {
	/// Built-in web search tool (e.g., Google Search for Gemini).
	#[display("WebSearch")]
	WebSearch,

	/// User-defined custom tool.
	#[from(String, &String, &str)]
	#[display("{_0}")]
	Custom(String),
}

impl ToolName {
	pub fn as_str(&self) -> &str {
		match self {
			Self::WebSearch => "WebSearch",
			// Future: other built-in tools (e.g., "Calculator", "CodeExecutor", etc.)
			Self::Custom(name) => name.as_str(),
		}
	}
}

impl AsRef<str> for ToolName {
	fn as_ref(&self) -> &str {
		self.as_str()
	}
}

// region:    --- Intos

impl From<ToolName> for String {
	fn from(name: ToolName) -> Self {
		match name {
			ToolName::Custom(n) => n,
			other => other.as_str().to_string(),
		}
	}
}

impl From<&ToolName> for String {
	fn from(name: &ToolName) -> Self {
		name.as_str().to_string()
	}
}
// endregion: --- Intos

// region:    --- JSON Serialize

impl Serialize for ToolName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(self.as_str())
	}
}

// endregion: --- JSON Serialize

/// Configuration variant for tools.
#[derive(Debug, Clone, Deserialize, From, PartialEq, Eq)]
pub enum ToolConfig {
	/// Configuration for web search.
	#[from]
	WebSearch(WebSearchConfig),

	/// Arbitrary JSON configuration for custom tools.
	#[from]
	Custom(serde_json::Value),
}

// region:    --- JSON Serialize

impl Serialize for ToolConfig {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			Self::WebSearch(conf) => conf.serialize(serializer),
			Self::Custom(conf) => conf.serialize(serializer),
		}
	}
}

// endregion: --- JSON Serialize
