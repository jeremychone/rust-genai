//! This module contains all the types related to a Chat Response (except ChatStream, which has its own file).

use serde::{Deserialize, Serialize};

use crate::chat::{ChatStream, MessageContent, ToolCall, Usage};
use crate::ModelIden;

// region:    --- ChatResponse

/// The Chat response when performing a direct `Client::`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	/// The eventual content of the chat response
	pub content: Option<MessageContent>,

	/// The eventual reasoning content,
	pub reasoning_content: Option<String>,

	/// The resolved Model Identifier (AdapterKind/ModelName) used for this request.
	/// > NOTE 1: This might be different from the request model if changed by the ModelMapper
	/// > NOTE 2: This might also be different than the used_model_iden as this will be the one returned by the AI Provider for this request
	pub model_iden: ModelIden,

	/// The provider model iden. Will be `model_iden` if not returned or mapped, but can be different.
	/// For example, `gpt-4o` model_iden might have a provider_model_iden as `gpt-4o-2024-08-06`
	pub provider_model_iden: ModelIden,

	// pub model
	/// The eventual usage of the chat response
	pub usage: Usage,
}

// Getters
impl ChatResponse {
	/// Returns the eventual content as `&str` if it is of type `MessageContent::Text`
	/// Otherwise, returns None
	pub fn content_text_as_str(&self) -> Option<&str> {
		self.content.as_ref().and_then(MessageContent::text_as_str)
	}

	/// Consumes the ChatResponse and returns the eventual String content of the `MessageContent::Text`
	/// Otherwise, returns None
	pub fn content_text_into_string(self) -> Option<String> {
		self.content.and_then(MessageContent::text_into_string)
	}

	pub fn tool_calls(&self) -> Option<Vec<&ToolCall>> {
		if let Some(MessageContent::ToolCalls(tool_calls)) = self.content.as_ref() {
			Some(tool_calls.iter().collect())
		} else {
			None
		}
	}

	pub fn into_tool_calls(self) -> Option<Vec<ToolCall>> {
		if let Some(MessageContent::ToolCalls(tool_calls)) = self.content {
			Some(tool_calls)
		} else {
			None
		}
	}
}

// endregion: --- ChatResponse

// region:    --- ChatStreamResponse

/// The result returned from the chat stream.
pub struct ChatStreamResponse {
	/// The stream result to iterate through the stream events
	pub stream: ChatStream,

	/// The Model Identifier (AdapterKind/ModelName) used for this request.
	pub model_iden: ModelIden,
}

// endregion: --- ChatStreamResponse
