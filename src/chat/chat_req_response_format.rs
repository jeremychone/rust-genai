use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Preferred response format for `ChatRequest` (structured output).
/// Only applied when the provider supports it.
///
/// > Note: Most providers currently ignore unsupported formats without error; this may change later.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	/// Request JSON-mode output (primarily for OpenAI).
	/// Note: Include an instruction like "Reply in JSON format." in your system/prompt to improve reliability.
	JsonMode,

	/// Request structured output.
	#[from]
	JsonSpec(JsonSpec),
}

/// JSON specification used to enforce structured output.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub struct JsonSpec {
	/// Specification name (primarily used by OpenAI).
	/// IMPORTANT: For OpenAI, only `-` and `_` are allowed; no spaces or other special characters.
	pub name: String,

	/// Human-readable description (used by some adapters in the future).
	/// NOTE: Currently ignored by the OpenAI adapter.
	pub description: Option<String>,

	/// Simplified JSON schema forwarded to the provider.
	pub schema: Value,
}

/// Constructors
impl JsonSpec {
	/// Create a `JsonSpec` with a name and schema.
	pub fn new(name: impl Into<String>, schema: impl Into<Value>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: schema.into(),
		}
	}
}

/// Setters
impl JsonSpec {
	/// Set an optional description (builder-style).
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}
}
