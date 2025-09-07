//! Types for chat responses. `ChatStream` is defined separately.

use serde::{Deserialize, Serialize};

use crate::ModelIden;
use crate::chat::{ChatStream, MessageContent, ToolCall, Usage};

// region:    --- ChatResponse

/// Response returned by a non-streaming chat request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	/// Message content returned by the assistant.
	pub content: MessageContent,

	/// Optional reasoning content returned by some models.
	pub reasoning_content: Option<String>,

	/// Resolved model identifier (AdapterKind/ModelName).
	/// > NOTE 1: May differ from the requested model after mapping.
	/// > NOTE 2: May also differ from `provider_model_iden` (provider-reported name).
	pub model_iden: ModelIden,

	/// Provider-reported model identifier.
	/// May differ from the requested or mapped `model_iden` (e.g., `gpt-4o` reported as `gpt-4o-2024-08-06`).
	/// Set explicitly by construction code; no implicit defaulting at the type level.
	pub provider_model_iden: ModelIden,

	// pub model
	/// Token usage reported by the provider.
	pub usage: Usage,

	/// Raw response body for provider-specific features.
	pub captured_raw_body: Option<serde_json::Value>,
}

// Getters
impl ChatResponse {
	/// Returns the first text segment, if any.
	pub fn first_text(&self) -> Option<&str> {
		self.content.first_text()
	}

	/// Consumes self and returns the first text segment, if any.
	pub fn into_first_text(self) -> Option<String> {
		self.content.into_first_text()
	}

	/// Returns all text segments (first per content item).
	pub fn texts(&self) -> Vec<&str> {
		self.content.texts()
	}

	/// Consumes self and returns all text segments (first per content item).
	pub fn into_texts(self) -> Vec<String> {
		self.content.into_texts()
	}

	/// Returns all captured tool calls.
	pub fn tool_calls(&self) -> Vec<&ToolCall> {
		self.content.tool_calls()
	}

	/// Consumes self and returns all captured tool calls.
	pub fn into_tool_calls(self) -> Vec<ToolCall> {
		self.content.into_tool_calls()
	}
}

/// Deprecated Getters
impl ChatResponse {
	/// Deprecated: use `first_text` or `texts`.
	/// Returns None if no text is present.
	#[deprecated(note = "Use '.first_text()' or '.texts()'")]
	pub fn content_text_as_str(&self) -> Option<&str> {
		self.first_text()
	}

	/// Deprecated: use `into_first_text` or `into_texts`.
	/// Returns None if no text is present.
	#[deprecated(note = "Use '.into_first_text()' or '.into_texts()")]
	pub fn content_text_into_string(self) -> Option<String> {
		self.into_first_text()
	}
}

// endregion: --- ChatResponse

// region:    --- ChatStreamResponse

/// Result of a streaming chat request.
pub struct ChatStreamResponse {
	/// Stream to iterate through response events.
	pub stream: ChatStream,

	/// Model identifier (AdapterKind/ModelName) used for this request.
	pub model_iden: ModelIden,
}

// endregion: --- ChatStreamResponse
