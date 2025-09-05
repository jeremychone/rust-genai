use serde::{Deserialize, Serialize};

/// Response produced by a tool invocation, paired with the originating tool call ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
	/// Identifier of the originating tool call.
	pub call_id: String,
	/// Tool output payload as a string. Providers may use JSON-serialized content.
	// For now, just a string (would probably be serialized JSON)
	pub content: String,
}

/// Constructor
impl ToolResponse {
	/// Creates a new ToolResponse with the provided tool_call_id and content.
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
