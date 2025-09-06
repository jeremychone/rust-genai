//! This module contains all the types related to a Chat Request (except ChatOptions, which has its own file).

use crate::chat::{ChatMessage, ChatRole, Tool};
use serde::{Deserialize, Serialize};

// region:    --- ChatRequest

/// Chat request for client chat calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
	/// The initial system content of the request.
	pub system: Option<String>,

	/// The messages of the request.
	pub messages: Vec<ChatMessage>,

	/// Optional tool definitions available to the model.
	pub tools: Option<Vec<Tool>>,
}

/// Constructors
impl ChatRequest {
	/// Construct from a set of messages.
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self {
			messages,
			system: None,
			tools: None,
		}
	}

	/// Construct with an initial system prompt.
	pub fn from_system(content: impl Into<String>) -> Self {
		Self {
			system: Some(content.into()),
			messages: Vec::new(),
			tools: None,
		}
	}

	/// Construct with a single user message.
	pub fn from_user(content: impl Into<String>) -> Self {
		Self {
			system: None,
			messages: vec![ChatMessage::user(content.into())],
			tools: None,
		}
	}

	/// Construct from messages.
	pub fn from_messages(messages: Vec<ChatMessage>) -> Self {
		Self {
			system: None,
			messages,
			tools: None,
		}
	}
}

/// Chainable Setters
impl ChatRequest {
	/// Set or replace the system prompt.
	pub fn with_system(mut self, system: impl Into<String>) -> Self {
		self.system = Some(system.into());
		self
	}

	/// Append one message.
	pub fn append_message(mut self, msg: impl Into<ChatMessage>) -> Self {
		self.messages.push(msg.into());
		self
	}

	/// Append multiple messages.
	pub fn append_messages(mut self, messages: Vec<ChatMessage>) -> Self {
		self.messages.extend(messages);
		self
	}

	/// Replace the tool set.
	pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
		self.tools = Some(tools);
		self
	}

	/// Append one tool.
	pub fn append_tool(mut self, tool: impl Into<Tool>) -> Self {
		self.tools.get_or_insert_with(Vec::new).push(tool.into());
		self
	}
}

/// Getters
impl ChatRequest {
	/// Iterate over all system content: the top-level system prompt, then any system-role messages.
	pub fn iter_systems(&self) -> impl Iterator<Item = &str> {
		self.system
			.iter()
			.map(|s| s.as_str())
			.chain(self.messages.iter().filter_map(|message| match message.role {
				ChatRole::System => message.content.first_text(),
				_ => None,
			}))
	}

	/// Concatenate all systems into one string,  
	/// keeping at 1 empty line in between
	pub fn combine_systems(&self) -> Option<String> {
		let mut systems: Option<String> = None;

		for system in self.iter_systems() {
			let systems_content = systems.get_or_insert_with(String::new);

			if !systems_content.is_empty() {
				// Single pass on the tail using byte slice patterns
				match systems_content.as_bytes() {
					// already ends with "\n\n" -> add nothing
					[.., b'\n', b'\n'] => {}
					// ends with a single '\n' -> add one more
					[.., b'\n'] => systems_content.push('\n'),
					// no trailing newline -> add two
					_ => systems_content.push_str("\n\n"),
				}
			}

			systems_content.push_str(system);
		}

		systems
	}
}

// endregion: --- ChatRequest
