// region:    --- ChatReq

#[derive(Debug, Clone)]
pub struct ChatRequest {
	pub messages: Vec<ChatMessage>,
}

impl ChatRequest {
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self { messages }
	}

	pub fn append_message(&mut self, msg: ChatMessage) -> &mut Self {
		self.messages.push(msg);
		self
	}
}

// endregion: --- ChatReq

// region:    --- ChatMsg

#[derive(Debug, Clone)]
pub struct ChatMessage {
	pub role: ChatRole,
	pub content: String,
	pub extra: Option<MessageExtra>,
}

#[derive(Debug, Clone)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

#[derive(Debug, Clone)]
pub enum MessageExtra {
	Tool(ToolExtra),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ToolExtra {
	tool_id: String,
}

/// Convenient constructors
impl ChatMessage {
	pub fn system(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			extra: None,
		}
	}

	pub fn assistant(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			extra: None,
		}
	}

	pub fn user(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			extra: None,
		}
	}
}

// endregion: --- ChatMsg
