use super::{ToolConfig, ToolName};
use crate::chat::CacheControl;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Tool metadata used for function calling by LLMs.
pub struct Tool {
	/// Normalized tool identifier.
	/// Example: `ToolName::Custom("get_weather".to_string())`.
	pub name: ToolName,

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

	/// Provider-native freeform custom-tool format.
	///
	/// OpenAI Responses uses values such as
	/// `{ "type": "grammar", "syntax": "lark", "definition": "..." }`.
	/// When present, the OpenAI Responses adapter emits a `type: "custom"`
	/// tool instead of a JSON-schema function tool.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub custom_format: Option<Value>,

	/// When `true`, the provider enforces strict schema validation on tool-call arguments.
	/// For OpenAI and Anthropic this sets `"strict": true` and sanitizes the schema for
	/// the provider's constrained-decoding dialect.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub strict: Option<bool>,

	/// Optional configuration for the tool.
	///
	/// Useful with embedded provider tools (e.g., Google Search for Gemini).
	pub config: Option<ToolConfig>,

	/// Optional cache control hint for prompt caching.
	/// Anthropic: set on the last tool to cache the entire tool list prefix.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cache_control: Option<CacheControl>,

	/// Anthropic fine-grained tool streaming (GA): when `true`, sets
	/// `eager_input_streaming` on the tool so the provider streams tool-call
	/// input without server-side JSON buffering (token-granular `input_json_delta`).
	#[serde(skip_serializing_if = "Option::is_none")]
	pub eager_input_streaming: Option<bool>,
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
		let mut size = match &self.name {
			ToolName::WebSearch => 9, // "WebSearch".len()
			ToolName::Custom(name) => name.len(),
		};
		size += self.description.as_ref().map(|d| d.len()).unwrap_or_default();
		size += self
			.schema
			.as_ref()
			.map(|s| serde_json::to_string(s).map(|j| j.len()).unwrap_or_default())
			.unwrap_or_default();
		size += self
			.custom_format
			.as_ref()
			.map(|f| serde_json::to_string(f).map(|j| j.len()).unwrap_or_default())
			.unwrap_or_default();
		size += self
			.config
			.as_ref()
			.map(|c| match c {
				ToolConfig::WebSearch(_) => 0,
				ToolConfig::Custom(v) => serde_json::to_string(v).map(|j| j.len()).unwrap_or_default(),
			})
			.unwrap_or_default();
		size
	}
}

/// Constructor
impl Tool {
	/// Create a new tool with the given name.
	pub fn new(name: impl Into<ToolName>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: None,
			custom_format: None,
			strict: None,
			config: None,
			cache_control: None,
			eager_input_streaming: None,
		}
	}

	/// Create a new web search tool.
	pub fn new_web_search() -> Self {
		Self::new(ToolName::WebSearch)
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

	/// Configure this as a provider-native freeform custom tool.
	pub fn with_custom_format(mut self, format: Value) -> Self {
		self.custom_format = Some(format);
		self
	}

	/// Enable strict schema validation for tool-call arguments on providers that support it.
	pub fn with_strict(mut self, strict: bool) -> Self {
		self.strict = Some(strict);
		self
	}

	/// Set provider-specific configuration (if any). Returns self for chaining.
	pub fn with_config(mut self, config: impl Into<ToolConfig>) -> Self {
		self.config = Some(config.into());
		self
	}

	/// Set cache control for prompt caching. Returns self for chaining.
	/// On Anthropic, set this on the last tool to cache the entire tool list prefix.
	pub fn with_cache_control(mut self, cache_control: impl Into<CacheControl>) -> Self {
		self.cache_control = Some(cache_control.into());
		self
	}

	/// Enable Anthropic fine-grained tool streaming (`eager_input_streaming`).
	/// The provider streams tool-call input without buffering the full JSON —
	/// useful for rendering tool arguments progressively as they arrive.
	pub fn with_eager_input_streaming(mut self, eager: bool) -> Self {
		self.eager_input_streaming = Some(eager);
		self
	}
}

// endregion: --- Setters
