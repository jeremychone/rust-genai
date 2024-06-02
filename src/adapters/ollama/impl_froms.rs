use crate::{ChatMsg, ChatRes, ChatRole};
use ollama_rs::generation::chat::{ChatMessage, ChatMessageResponse, MessageRole};

// region:    --- From [genai] to [raw]

impl From<ChatMsg> for ChatMessage {
	fn from(value: ChatMsg) -> Self {
		Self {
			role: value.role.into(),
			content: value.content,
			images: None,
		}
	}
}

impl From<ChatRole> for MessageRole {
	fn from(value: ChatRole) -> Self {
		match value {
			ChatRole::User => Self::User,
			ChatRole::Assistant => Self::Assistant,
			ChatRole::System => Self::System,
			// TODO: For Tool, needs to decide what to do in this case until we support tools in ollama
			ChatRole::Tool => Self::System,
		}
	}
}

// endregion: --- From [genai] to [raw]

// region:    --- From [raw] to [genai]

impl From<ChatMessageResponse> for ChatRes {
	fn from(value: ChatMessageResponse) -> Self {
		let response = value.message.map(|m| m.content);
		ChatRes { response }
	}
}

// endregion: --- From [raw] to [genai]
