use super::ToolCall;
use crate::chat::Binary;
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
	/// Optional binary attachments produced by the tool (e.g., screenshots, file reads).
	///
	/// Image parts serialize natively where the wire supports them (Anthropic
	/// `tool_result`, Bedrock Converse `toolResult`, OpenAI Responses
	/// `function_call_output`) and ride in a follow-up user message elsewhere
	/// (OpenAI Chat Completions-compatible providers, Gemini, Ollama).
	/// Non-image parts are currently skipped with a warning.
	/// Text-only responses (no `parts`) keep their exact legacy serialization.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub parts: Option<Vec<Binary>>,
}

/// Constructor
impl ToolResponse {
	/// Creates a new ToolResponse with the provided tool_call_id and content.
	pub fn new(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
		Self {
			call_id: tool_call_id.into(),
			fn_name: None,
			content: content.into(),
			parts: None,
		}
	}

	/// Creates a ToolResponse from the originating ToolCall.
	pub fn from_tool_call(tool_call: &ToolCall, content: impl Into<String>) -> Self {
		Self {
			call_id: tool_call.call_id.clone(),
			fn_name: Some(tool_call.fn_name.clone()),
			content: content.into(),
			parts: None,
		}
	}

	/// Attach the function/tool name to this response.
	pub fn with_fn_name(mut self, fn_name: impl Into<String>) -> Self {
		self.fn_name = Some(fn_name.into());
		self
	}

	/// Set the binary attachments of this response. Returns self for chaining.
	pub fn with_parts<I>(mut self, parts: I) -> Self
	where
		I: IntoIterator,
		I::Item: Into<Binary>,
	{
		self.parts = Some(parts.into_iter().map(Into::into).collect());
		self
	}

	/// Append a single binary attachment to this response. Returns self for chaining.
	pub fn append_binary(mut self, binary: impl Into<Binary>) -> Self {
		self.parts.get_or_insert_with(Vec::new).push(binary.into());
		self
	}
}

/// Computed accessors
impl ToolResponse {
	/// Returns an approximate in-memory size of this `ToolResponse`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `call_id`
	/// - `fn_name` (if any)
	/// - `content`
	/// - plus the `Binary::size()` of each part (if any)
	pub fn size(&self) -> usize {
		let parts_size: usize = self
			.parts
			.as_ref()
			.map(|parts| parts.iter().map(Binary::size).sum())
			.unwrap_or_default();
		self.call_id.len() + self.fn_name.as_ref().map(|name| name.len()).unwrap_or(0) + self.content.len() + parts_size
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

	fn parts(&self) -> Option<&[Binary]> {
		self.parts.as_deref()
	}
}

// region:    --- Tests

#[cfg(test)]
#[path = "tool_response_tests.rs"]
mod tests;

// endregion: --- Tests
