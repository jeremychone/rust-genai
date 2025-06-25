//! This module contains all the types related to a Chat Response (except ChatStream, which has its own file).

use serde::{Deserialize, Serialize};

use crate::ModelIden;
use crate::chat::{ChatStream, MessageContent, ToolCall, Usage};

// region:    --- ChatResponse

/// The Chat response when performing a direct `Client::`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	/// The eventual content of the chat response
	pub content: Vec<MessageContent>,

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
	/// Returns a reference to the first text content if available.
	pub fn first_text(&self) -> Option<&str> {
		for content_item in &self.content {
			if let MessageContent::Text(content) = content_item {
				return Some(content);
			}
		}
		None
	}

	/// Consumes the `ChatResponse` and returns the first text content if available.
	pub fn into_first_text(self) -> Option<String> {
		for content_item in self.content {
			if let MessageContent::Text(content) = content_item {
				return Some(content);
			}
		}
		None
	}

	/// Returns a vector of references to all text content parts.
	pub fn texts(&self) -> Vec<&str> {
		let mut all_texts = Vec::new();
		for content_item in &self.content {
			if let MessageContent::Text(content) = content_item {
				all_texts.push(content.as_str());
			}
		}
		all_texts
	}

	/// Consumes the `ChatResponse` and returns a vector of all text content parts.
	pub fn into_texts(self) -> Vec<String> {
		let mut all_texts = Vec::new();
		for content_item in self.content {
			if let MessageContent::Text(content) = content_item {
				all_texts.push(content);
			}
		}
		all_texts
	}

	/// Returns a vector of references to all captured tool calls.
	pub fn tool_calls(&self) -> Vec<&ToolCall> {
		let mut all_tool_calls: Vec<&ToolCall> = Vec::new();
		for content_item in &self.content {
			if let MessageContent::ToolCalls(tool_calls) = content_item {
				// TODO: Avoid the extra Vec alloc
				let tool_calls: Vec<&ToolCall> = tool_calls.iter().collect();
				all_tool_calls.extend(tool_calls);
			}
		}

		all_tool_calls
	}

	/// Consumes the `ChatResponse` and returns a vector of all captured tool calls.
	pub fn into_tool_calls(self) -> Vec<ToolCall> {
		let mut all_tool_calls: Vec<ToolCall> = Vec::new();
		for content_item in self.content {
			if let MessageContent::ToolCalls(tool_calls) = content_item {
				all_tool_calls.extend(tool_calls);
			}
		}

		all_tool_calls
	}
}

/// Deprecated Getters
impl ChatResponse {
	/// Returns the eventual content as `&str` if it is of type `MessageContent::Text`
	/// Otherwise, returns None
	#[deprecated(note = "Use '.first_text()' or '.texts()")]
	pub fn content_text_as_str(&self) -> Option<&str> {
		self.first_text()
	}

	/// Consumes the ChatResponse and returns the eventual String content of the `MessageContent::Text`
	/// Otherwise, returns None
	#[deprecated(note = "Use '.into_first_text()' or '.into_texts()")]
	pub fn content_text_into_string(self) -> Option<String> {
		self.into_first_text()
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

