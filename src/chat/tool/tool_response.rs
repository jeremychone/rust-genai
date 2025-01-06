use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
	pub call_id: String,
	// For now, just a string (would probably be serialized JSON)
	pub content: String,
}

/// Constructor
impl ToolResponse {
	pub fn new(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
		Self {
			call_id: tool_call_id.into(),
			content: content.into(),
		}
	}
}

/// Getters
#[allow(unused)]
impl ToolResponse {
	fn tool_call_id(&self) -> &str {
		&self.call_id
	}

	fn content(&self) -> &str {
		&self.content
	}
}
