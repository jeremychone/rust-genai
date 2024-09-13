use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The chat response format to be sent back by the LLM.
/// This will be taken into consideration only if the provider supports it.
///
/// > Note: Currently, the AI Providers will not report an error if not supported. It will just be ignored.
/// >       This may change in the future.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	/// Request to return a well-formatted JSON. Mostly for OpenAI.
	/// Note: Make sure to add "Reply in JSON format." to the prompt or system to ensure it works correctly.
	JsonMode,

	/// Request to return a stuctured Structured Output
	#[from]
	JsonSpec(JsonSpec),
}

/// The json specification for the Structured Output format.
#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub struct JsonSpec {
	/// The name of the spec. Mostly used by open-ai.
	/// IMPORTANT: With openAI, this cannot contains any space or special characters beside `-` and `_`
	pub name: String,
	/// The description of the json spec. Mostly used by OpenAI adapters (future).
	/// NOTE: Today, ignored in the OpenAI adapter
	pub description: Option<String>,

	/// The simplified json-schema that will be used by the AI Provider as json schema.
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
	/// Chainable setter to set the descrition in a JsonSpec construct.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}
}
