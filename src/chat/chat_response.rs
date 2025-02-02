//! This module contains all the types related to a Chat Response (except ChatStream, which has its own file).

use serde::{Deserialize, Serialize};

use crate::chat::{ChatStream, MessageContent, ToolCall};
use crate::ModelIden;
use serde_with::{serde_as, skip_serializing_none};

// region:    --- ChatResponse

/// The Chat response when performing a direct `Client::`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
	/// The eventual content of the chat response
	pub content: Option<MessageContent>,

	/// The eventual reasoning content,
	pub reasoning_content: Option<String>,

	/// The Model Identifier (AdapterKind/ModelName) used for this request.
	/// > NOTE: This might be different from the request model if changed by the ModelMapper
	pub model_iden: ModelIden,

	/// The eventual usage of the chat response
	pub usage: MetaUsage,
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

// region:    --- MetaUsage

/// IMPORTANT: This is **NOT SUPPORTED** for now. To indicate the API direction.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MetaUsage {
	/// The input tokens (replaces input_tokens)
	pub prompt_tokens: Option<i32>,
	pub prompt_tokens_details: Option<PromptTokensDetails>,

	/// The completions/output tokens
	pub completion_tokens: Option<i32>,
	pub completion_tokens_details: Option<CompletionTokensDetails>,

	/// The total number of tokens if returned by the API call.
	/// This will either be the total_tokens if returned, or the sum of prompt/completion if not specified in the response.
	pub total_tokens: Option<i32>,

	// -- Deprecated
	/// The number of input tokens if returned by the API call.
	#[deprecated(note = "Use prompt_tokens (for now it is a clone, but later will be removed)")]
	#[serde(skip)]
	pub input_tokens: Option<i32>,

	/// The number of output tokens if returned by the API call.
	#[deprecated(note = "Use prompt_tokens (for now it is a clone, but later will be removed)")]
	#[serde(skip)]
	pub output_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
	cached_tokens: Option<i32>,
	audio_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
	accepted_prediction_tokens: Option<i32>,
	rejected_prediction_tokens: Option<i32>,
	reasoning_tokens: Option<i32>,
	audio_tokens: Option<i32>,
}

// endregion: --- MetaUsage
