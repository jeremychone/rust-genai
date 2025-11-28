use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Tool metadata used for function calling by LLMs.
pub struct Tool {
	/// Tool name, typically the function name.
	/// Example: `get_weather`.
	pub name: String,

	/// Human-readable description used by the LLM to understand when and how to call this tool.
	pub description: Option<String>,

	/// JSON Schema for the tool parameters.
	/// Example:
	/// ```rust
	/// json!({
	/// "type": "object",
	/// "properties": {
	///    "city": {
	///        "type": "string",
	///        "description": "The city name"
	///    },
	///    "country": {
	///        "type": "string",
	///        "description": "The most likely country of this city name"
	///    },
	///    "unit": {
	///        "type": "string",
	///        "enum": ["C", "F"],
	///        "description": "The temperature unit for the country. C for Celsius, and F for Fahrenheit"
	///    }
	/// },
	/// "required": ["city", "country", "unit"],
	/// })
	/// ```
	pub schema: Option<Value>,

	/// Optional configuration for the tool.
	///
	/// Useful with embedded provider tools (e.g., Google Search for Gemini).
	pub config: Option<Value>,
}

/// Computed accessors
impl Tool {
	/// Returns an approximate in-memory size of this `Tool`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `name`
	/// - `description` (if any)
	/// - JSON-serialized `schema` (if any)
	/// - JSON-serialized `config` (if any)
	pub fn size(&self) -> usize {
		let mut size = self.name.len();
		size += self.description.as_ref().map(|d| d.len()).unwrap_or_default();
		size += self
			.schema
			.as_ref()
			.map(|s| serde_json::to_string(s).map(|j| j.len()).unwrap_or_default())
			.unwrap_or_default();
		size += self
			.config
			.as_ref()
			.map(|c| serde_json::to_string(c).map(|j| j.len()).unwrap_or_default())
			.unwrap_or_default();
		size
	}
}

/// Constructor
impl Tool {
	/// Create a new tool with the given name.
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: None,
			config: None,
		}
	}
}

// region:    --- Setters

impl Tool {
	/// Set the tool description. Returns self for chaining.
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Set the JSON Schema for the tool parameters. Returns self for chaining.
	pub fn with_schema(mut self, parameters: Value) -> Self {
		self.schema = Some(parameters);
		self
	}

	/// Set provider-specific configuration (if any). Returns self for chaining.
	pub fn with_config(mut self, config: Value) -> Self {
		self.config = Some(config);
		self
	}
}

// endregion: --- Setters
