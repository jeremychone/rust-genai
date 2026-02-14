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

#[cfg(test)]
mod tests {
	use super::*;

	/// ToolName::Custom serializes as a bare string (e.g. `"get_weather"`)
	/// via the custom Serialize impl, but the derived Deserialize expects
	/// either the literal variant name `"WebSearch"` or the externally-tagged
	/// form `{"Custom":"get_weather"}`.
	///
	/// This means any ToolName::Custom value cannot survive a JSON round-trip.
	#[test]
	fn tool_name_custom_roundtrip() {
		let original = ToolName::Custom("get_weather".into());

		let json = serde_json::to_string(&original).unwrap();
		// Custom Serialize writes it as a bare string:
		assert_eq!(json, r#""get_weather""#);

		// Derived Deserialize cannot parse it back:
		//   "unknown variant `get_weather`, expected `WebSearch` or `Custom`"
		let round: Result<ToolName, _> = serde_json::from_str(&json);
		assert!(
			round.is_ok(),
			"ToolName::Custom should round-trip: {}",
			round.unwrap_err()
		);
	}

	/// WebSearch round-trips fine because the variant name matches.
	#[test]
	fn tool_name_websearch_roundtrip() {
		let original = ToolName::WebSearch;

		let json = serde_json::to_string(&original).unwrap();
		assert_eq!(json, r#""WebSearch""#);

		let round: ToolName = serde_json::from_str(&json).unwrap();
		assert_eq!(round, ToolName::WebSearch);
	}

	/// ToolConfig::Custom serializes the inner Value directly (e.g. `{"key":"val"}`),
	/// but the derived Deserialize expects the externally-tagged form
	/// `{"Custom":{"key":"val"}}`.
	#[test]
	fn tool_config_custom_roundtrip() {
		let original = ToolConfig::Custom(serde_json::json!({"key": "value"}));

		let json = serde_json::to_string(&original).unwrap();
		// Custom Serialize flattens to the inner value:
		assert_eq!(json, r#"{"key":"value"}"#);

		// Derived Deserialize cannot parse it back:
		let round: Result<ToolConfig, _> = serde_json::from_str(&json);
		assert!(
			round.is_ok(),
			"ToolConfig::Custom should round-trip: {}",
			round.unwrap_err()
		);
	}
}
