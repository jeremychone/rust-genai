use crate::{ChatMessage, ChatResponse, ChatRole, Error, LegacyClientKind, StreamItem};
use ollama_rs::generation::chat as ol_chat;

// region:    --- From [genai] to [raw]

impl From<ChatMessage> for ol_chat::ChatMessage {
	fn from(value: ChatMessage) -> Self {
		Self {
			role: value.role.into(),
			content: value.content,
			images: None,
		}
	}
}

impl From<ChatRole> for ol_chat::MessageRole {
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

impl From<ol_chat::ChatMessageResponse> for ChatResponse {
	fn from(value: ol_chat::ChatMessageResponse) -> Self {
		let response = value.message.map(|m| m.content);
		ChatResponse { content: response }
	}
}

impl From<ol_chat::ChatMessageResponse> for StreamItem {
	fn from(value: ol_chat::ChatMessageResponse) -> Self {
		let response = value.message.map(|m| m.content);

		StreamItem { content: response }
	}
}

// endregion: --- From [raw] to [genai]

impl From<ollama_rs::error::OllamaError> for Error {
	fn from(raw_client_error: ollama_rs::error::OllamaError) -> Self {
		Self::provider_connector(LegacyClientKind::OllamaRs, raw_client_error.to_string())
	}
}
