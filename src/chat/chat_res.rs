//! This module contains all the types related to a Chat Response (except ChatStream, which has its own file).

use serde::{Deserialize, Serialize};

use crate::chat::{ChatStream, MessageContent};
use crate::ModelIden;

// region:    --- ChatResponse

/// The Chat response when performing a direct `Client::`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	/// The eventual content of the chat response
	pub content: Option<MessageContent>,

	/// The eventual usage of the chat response
	pub usage: MetaUsage,

	/// The Model Identifier (AdapterKind/ModelName) used for this request.
	/// > NOTE: This might be different from the request model if changed by the ModelMapper
	pub model_iden: ModelIden,
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

// region:    --- MetaUsage

/// IMPORTANT: This is **NOT SUPPORTED** for now. To indicate the API direction.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MetaUsage {
	/// The number of input tokens if returned by the API call.
	pub input_tokens: Option<i32>,
	/// The number of output tokens if returned by the API call.
	pub output_tokens: Option<i32>,
	/// The total number of tokens if returned by the API call.
	/// This will either be the total_tokens if returned, or the sum of input/output if not specified in the response.
	pub total_tokens: Option<i32>,
}

// endregion: --- MetaUsage