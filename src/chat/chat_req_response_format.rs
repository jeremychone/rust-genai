use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The chat response format for the ChatRequest for structured output.
/// This will be taken into consideration only if the provider supports it.
///
/// > Note: Currently, the AI Providers will not report an error if not supported. It will just be ignored.
/// >       This may change in the future.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	/// Request to return a well-formatted JSON. Mostly for OpenAI.
	/// Note: Make sure to add "Reply in JSON format." to the prompt or system to ensure it works correctly.
	JsonMode,

	/// Request to return a structured output.
	#[from]
	JsonSpec(JsonSpec),
}

/// The JSON specification for the structured output format.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub struct JsonSpec {
	/// The name of the spec. Mostly used by OpenAI.
	/// IMPORTANT: With OpenAI, this cannot contain any spaces or special characters besides `-` and `_`.
	pub name: String,
	/// The description of the JSON spec. Mostly used by OpenAI adapters (future).
	/// NOTE: Currently ignored in the OpenAI adapter.
	pub description: Option<String>,

	/// The simplified JSON schema that will be used by the AI provider as JSON schema.
	pub schema: Value,
}

/// Constructors
impl JsonSpec {
	/// Create a new JsonSpec from name and schema.
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
	/// Chainable setter to set the description in a JsonSpec construct.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}
}
