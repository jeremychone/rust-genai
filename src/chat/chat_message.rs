use crate::chat::{MessageContent, ToolCall, ToolResponse};
use serde::{Deserialize, Serialize};

/// An individual chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The role of the message.
	pub role: ChatRole,

	/// The content of the message.
	pub content: MessageContent,
}

/// Constructors
impl ChatMessage {
	/// Create a new ChatMessage with the role `ChatRole::System`.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::Assistant`.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::User`.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
		}
	}
}

/// Chat roles.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

// region:    --- Froms

impl From<Vec<ToolCall>> for ChatMessage {
	fn from(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: MessageContent::from(tool_calls),
		}
	}
}

impl From<ToolResponse> for ChatMessage {
	fn from(value: ToolResponse) -> Self {
		Self {
			role: ChatRole::Tool,
			content: MessageContent::from(value),
		}
	}
}

// endregion: --- Froms
