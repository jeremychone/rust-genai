use serde::{Deserialize, Serialize};

/// Provider-neutral tool selection preference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
	/// Let the model decide whether to call tools.
	Auto,
	/// Prevent tool calls.
	None,
	/// Require the model to call at least one available tool.
	Required,
	/// Require the model to call a specific tool.
	Tool { name: String },
}

impl ToolChoice {
	/// Require a specific tool by name.
	pub fn tool(name: impl Into<String>) -> Self {
		Self::Tool { name: name.into() }
	}

	pub(crate) fn tool_name(&self) -> Option<&str> {
		match self {
			Self::Tool { name } => Some(name.as_str()),
			_ => None,
		}
	}
}
