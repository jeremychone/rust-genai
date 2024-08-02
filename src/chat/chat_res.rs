//! This module contains all the types related to a Chat Response (except ChatStream which has it file).

use crate::chat::{ChatStream, MessageContent};

// region:    --- ChatResponse

#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
	pub content: Option<MessageContent>,
	pub usage: MetaUsage,
}

// Getters
impl ChatResponse {
	/// Returns the eventual content as `&str` if it is of type `MessageContent::Text`
	/// Otherwise, return None
	pub fn content_text_as_str(&self) -> Option<&str> {
		self.content.as_ref().and_then(MessageContent::text_as_str)
	}

	/// Consume the ChatResponse and returns the eventual String content of the `MessageContent::Text`
	/// Otherwise, return None
	pub fn content_text_into_string(self) -> Option<String> {
		self.content.and_then(MessageContent::text_into_string)
	}
}

// endregion: --- ChatResponse

// region:    --- ChatStreamResponse

pub struct ChatStreamResponse {
	pub stream: ChatStream,
}

// endregion: --- ChatStreamResponse

// region:    --- MetaUsage

/// IMPORTANT: This is **NOT SUPPORTED** for now. To show the API direction.
#[derive(Default, Debug, Clone)]
pub struct MetaUsage {
	pub input_tokens: Option<i32>,
	pub output_tokens: Option<i32>,
	pub total_tokens: Option<i32>,
}

// endregion: --- MetaUsage
