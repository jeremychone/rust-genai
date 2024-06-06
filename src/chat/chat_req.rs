// region:    --- ChatRequest

#[derive(Debug, Clone, Default)]
pub struct ChatRequest {
	pub system: Option<String>,
	pub messages: Vec<ChatMessage>,
}

/// Constructors
impl ChatRequest {
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self { messages, system: None }
	}
}

/// Setters (builder style)
impl ChatRequest {
	pub fn with_system(mut self, system: impl Into<String>) -> Self {
		self.system = Some(system.into());
		self
	}

	pub fn append_message(mut self, msg: ChatMessage) -> Self {
		self.messages.push(msg);
		self
	}
}

// endregion: --- ChatRequest

// region:    --- ChatMessage

#[derive(Debug, Clone)]
pub struct ChatMessage {
	pub role: ChatRole,
	pub content: String,
	pub extra: Option<MessageExtra>,
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

// endregion: --- ChatMessage
