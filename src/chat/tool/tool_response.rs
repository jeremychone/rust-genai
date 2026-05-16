use super::ToolCall;
use serde::{Deserialize, Serialize};

/// Response produced by a tool invocation, paired with the originating tool call ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
	/// Identifier of the originating tool call.
	pub call_id: String,
	/// Name of the function/tool that produced this response.
	///
	/// Most providers correlate responses by call ID, but Gemini's
	/// `functionResponse.name` expects the function name.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fn_name: Option<String>,
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
			fn_name: None,
			content: content.into(),
		}
	}

	/// Creates a ToolResponse from the originating ToolCall.
	pub fn from_tool_call(tool_call: &ToolCall, content: impl Into<String>) -> Self {
		Self {
			call_id: tool_call.call_id.clone(),
			fn_name: Some(tool_call.fn_name.clone()),
			content: content.into(),
		}
	}

	/// Attach the function/tool name to this response.
	pub fn with_fn_name(mut self, fn_name: impl Into<String>) -> Self {
		self.fn_name = Some(fn_name.into());
		self
	}
}

/// Computed accessors
impl ToolResponse {
	/// Returns an approximate in-memory size of this `ToolResponse`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `call_id`
	/// - `content`
	pub fn size(&self) -> usize {
		self.call_id.len() + self.fn_name.as_ref().map(|name| name.len()).unwrap_or(0) + self.content.len()
	}
}

/// Getters
#[allow(unused)]
impl ToolResponse {
	fn tool_call_id(&self) -> &str {
		&self.call_id
	}

	fn fn_name(&self) -> Option<&str> {
		self.fn_name.as_deref()
	}

	fn content(&self) -> &str {
		&self.content
	}
}
