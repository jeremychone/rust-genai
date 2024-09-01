//! This module contains all the types related to a Chat Request (except ChatOptions, which has its own file).

use crate::chat::MessageContent;
use serde::{Serialize, Deserialize};

// region:    --- ChatRequest

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
	pub system: Option<String>,
	pub messages: Vec<ChatMessage>,
}

/// Constructors
impl ChatRequest {
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self { messages, system: None }
	}

	/// From `.system` property content.
	pub fn from_system(content: impl Into<String>) -> Self {
		Self {
			system: Some(content.into()),
			messages: Vec::new(),
		}
	}
}

/// Chainable Setters
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

/// Getters
impl ChatRequest {
	/// Iterate through all of the system content, starting with the eventual
	/// ChatRequest.system and then the ChatMessage of role System
	pub fn iter_systems(&self) -> impl Iterator<Item = &str> {
		self.system
			.iter()
			.map(|s| s.as_str())
			.chain(self.messages.iter().filter_map(|message| match message.role {
				ChatRole::System => match message.content {
					MessageContent::Text(ref content) => Some(content.as_str()),
				},
				_ => None,
			}))
	}

	/// Combine the eventual ChatRequest `.system` and system messages into one string.
	/// - It will start with the evnetual `chat_request.system`
	/// - Then concatenate the eventual `ChatRequestMessage` of Role `System`
	/// - This will attempt to add an empty line between system content. So, it will add
	///   - Two `\n` when the prev content does not end with `\n`
	///   - and one `\n` if the prev content ends with `\n`
	pub fn combine_systems(&self) -> Option<String> {
		let mut systems: Option<String> = None;

		for system in self.iter_systems() {
			let systems_content = systems.get_or_insert_with(|| "".to_string());

			// add eventual separator
			if systems_content.ends_with('\n') {
				systems_content.push('\n');
			} else if !systems_content.is_empty() {
				systems_content.push_str("\n\n");
			} // do not add any empyt line if prev content is empty

			systems_content.push_str(system);
		}

		systems
	}
}

// endregion: --- ChatRequest

// region:    --- ChatMessage

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	pub role: ChatRole,
	pub content: MessageContent,
	pub extra: Option<MessageExtra>,
}

/// Constructors
impl ChatMessage {
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			extra: None,
		}
	}

	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			extra: None,
		}
	}

	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			extra: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageExtra {
	Tool(ToolExtra),
}

#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExtra {
	tool_id: String,
}

// endregion: --- ChatMessage
