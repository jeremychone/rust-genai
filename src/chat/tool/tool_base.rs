use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
	/// The tool name, which is typically the function name
	/// e.g., `get_weather`
	pub name: String,

	/// The description of the tool which will be used by the LLM to understand the context/usage of this tool
	pub description: Option<String>,

	/// The json-schema for the parameters
	/// e.g.,
	/// ```json
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
	///        "description": "The temperature unit of the country. C for Celsius, and F for Fahrenheit"
	///    }
	/// },
	/// "required": ["city", "country", "unit"],
	/// })
	/// ```
	pub schema: Option<Value>,
}

/// Constructor
impl Tool {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: None,
		}
	}
}

// region:    --- Setters

impl Tool {
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	pub fn with_schema(mut self, parameters: Value) -> Self {
		self.schema = Some(parameters);
		self
	}
}

// endregion: --- Setters
