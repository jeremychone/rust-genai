use crate::{ChatMsg, ChatRes, ChatRole};
use async_openai::types::{
	ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, CreateChatCompletionResponse, Role,
};

// region:    --- From [genai] to [raw]

/// From ChatMsg to async_openai ChatCompletionRequestMessage
impl From<ChatMsg> for ChatCompletionRequestMessage {
	fn from(chat_msg: ChatMsg) -> Self {
		let role = chat_msg.role.into();
		let content = chat_msg.content;

		match chat_msg.extra {
			None => ChatCompletionRequestUserMessage {
				content: content.into(),
				role,
				name: None,
			}
			.into(),
			_ => todo!("chat_msg other not supported yet"),
		}
	}
}

/// From ChatRole to async_openai Role
impl From<ChatRole> for Role {
	fn from(chat_role: ChatRole) -> Self {
		match chat_role {
			ChatRole::System => Role::System,
			ChatRole::User => Role::User,
			ChatRole::Assistant => Role::Assistant,
			ChatRole::Tool => Role::Tool,
			// TODO: need to decide what to do with function
		}
	}
}

// endregion: --- From [genai] to [raw]

// region:    --- From [raw] to [genai]

impl From<CreateChatCompletionResponse> for ChatRes {
	fn from(raw_res: CreateChatCompletionResponse) -> Self {
		// NOTE: For now, very basic, just first choice.
		let choice = raw_res.choices.into_iter().next();
		let response = choice.and_then(|choice| choice.message.content);

		ChatRes { response }
	}
}

// endregion: --- From [raw] to [genai]
