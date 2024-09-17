//! This module contains all the types related to a Chat Request (except ChatOptions, which has its own file).

use crate::chat::MessageContent;
use serde::{Deserialize, Serialize};

// region:    --- ChatRequest

/// The Chat request when performing a direct `Client::`
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
	/// The initial system of the request.
	pub system: Option<String>,

	/// The messages of the request.
	pub messages: Vec<ChatMessage>,
}

/// Constructors
impl ChatRequest {
	/// Create a new ChatRequest with the given messages.
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self { messages, system: None }
	}

	/// From the `.system` property content.
	pub fn from_system(content: impl Into<String>) -> Self {
		Self {
			system: Some(content.into()),
			messages: Vec::new(),
		}
	}
}

/// Chainable Setters
impl ChatRequest {
	/// Set the system content of the request.
	pub fn with_system(mut self, system: impl Into<String>) -> Self {
		self.system = Some(system.into());
		self
	}

	/// Append a message to the request.
	pub fn append_message(mut self, msg: ChatMessage) -> Self {
		self.messages.push(msg);
		self
	}
}

/// Getters
impl ChatRequest {
	/// Iterate through all of the system content, starting with the eventual
	/// ChatRequest.system and then the ChatMessage of role System.
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
	/// - It will start with the eventual `chat_request.system`.
	/// - Then concatenate the eventual `ChatRequestMessage` of Role `System`.
	/// - This will attempt to add an empty line between system content. So, it will add
	///   - Two `\n` when the previous content does not end with `\n`.
	///   - One `\n` if the previous content ends with `\n`.
	pub fn combine_systems(&self) -> Option<String> {
		let mut systems: Option<String> = None;

		for system in self.iter_systems() {
			let systems_content = systems.get_or_insert_with(|| "".to_string());

			// add eventual separator
			if systems_content.ends_with('\n') {
				systems_content.push('\n');
			} else if !systems_content.is_empty() {
				systems_content.push_str("\n\n");
			} // do not add any empty line if previous content is empty

			systems_content.push_str(system);
		}

		systems
	}
}

// endregion: --- ChatRequest

// region:    --- ChatMessage

/// An individual chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The role of the message.
	pub role: ChatRole,

	/// The content of the message.
	pub content: MessageContent,

	/// Extra information about the message.
	pub extra: Option<MessageExtra>,
}

/// Constructors
impl ChatMessage {
	/// Create a new ChatMessage with the role `ChatRole::System`.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			extra: None,
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::Assistant`.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			extra: None,
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::User`.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			extra: None,
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

/// NOTE: DO NOT USE, just a placeholder for now.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MessageExtra {
	Tool(ToolExtra),
}

/// NOTE: DO NOT USE, just a placeholder for now.
#[allow(unused)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExtra {
	tool_id: String,
}

// endregion: --- ChatMessage
