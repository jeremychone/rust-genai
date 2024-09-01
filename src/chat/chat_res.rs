//! This module contains all the types related to a Chat Response (except ChatStream which has it file).

use serde::{Deserialize, Serialize};

use crate::chat::{ChatStream, MessageContent};
use crate::ModelIden;

// region:    --- ChatResponse

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	pub content: Option<MessageContent>,
	pub usage: MetaUsage,
	pub model_iden: ModelIden,
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
	pub model_iden: ModelIden,
}

// endregion: --- ChatStreamResponse

// region:    --- MetaUsage

/// IMPORTANT: This is **NOT SUPPORTED** for now. To show the API direction.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MetaUsage {
	pub input_tokens: Option<i32>,
	pub output_tokens: Option<i32>,
	pub total_tokens: Option<i32>,
}

// endregion: --- MetaUsage
