use crate::chat::{ChatMessage, ChatRole, StreamEnd, Tool, ToolCall, ToolResponse};
use crate::support;
use serde::{Deserialize, Serialize};

// region:    --- ChatRequest

/// A provider-neutral chat request containing conversation messages and available tools.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatRequest {
	/// The initial system content of the request.
	pub system: Option<String>,

	/// The messages of the request.
	#[serde(default)]
	pub messages: Vec<ChatMessage>,

	/// Optional tool definitions available to the model.
	pub tools: Option<Vec<Tool>>,

	/// Previous response ID used to continue a provider-managed stateful session.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub previous_response_id: Option<String>,

	/// Whether the provider should store the response for stateful continuation.
	///
	/// An unset value does not opt in to storage.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub store: Option<bool>,
}

/// Constructors
impl ChatRequest {
	/// Construct from a set of messages.
	pub fn new(messages: Vec<ChatMessage>) -> Self {
		Self {
			messages,
			system: None,
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct with an initial system prompt.
	pub fn from_system(content: impl Into<String>) -> Self {
		Self {
			system: Some(content.into()),
			messages: Vec::new(),
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct with a single user message.
	pub fn from_user(content: impl Into<String>) -> Self {
		Self {
			system: None,
			messages: vec![ChatMessage::user(content.into())],
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}

	/// Construct from messages.
	pub fn from_messages(messages: Vec<ChatMessage>) -> Self {
		Self {
			system: None,
			messages,
			tools: None,
			previous_response_id: None,
			store: None,
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

	/// Append multiple messages from any iterable.
	pub fn append_messages<I>(mut self, messages: I) -> Self
	where
		I: IntoIterator,
		I::Item: Into<ChatMessage>,
	{
		self.messages.extend(messages.into_iter().map(Into::into));
		self
	}

	/// Replace the tool set.
	pub fn with_tools<I>(mut self, tools: I) -> Self
	where
		I: IntoIterator,
		I::Item: Into<Tool>,
	{
		self.tools = Some(tools.into_iter().map(Into::into).collect());
		self
	}

	/// Set the previous response ID for stateful sessions.
	pub fn with_previous_response_id(mut self, previous_response_id: impl Into<String>) -> Self {
		self.previous_response_id = Some(previous_response_id.into());
		self
	}

	/// Set whether to store the response for stateful sessions.
	pub fn with_store(mut self, store: bool) -> Self {
		self.store = Some(store);
		self
	}

	/// Append one tool.
	pub fn append_tool(mut self, tool: impl Into<Tool>) -> Self {
		self.tools.get_or_insert_with(Vec::new).push(tool.into());
		self
	}

	/// Appends a captured assistant tool-use turn followed by its tool response.
	///
	/// Captured content is preferred because it preserves part ordering. If no assistant
	/// content or tool calls were captured, only the tool response is appended.
	pub fn append_tool_use_from_stream_end(mut self, end: &StreamEnd, tool_response: ToolResponse) -> Self {
		if let Some(content) = &end.captured_content {
			// Use captured content directly (contains thoughts/text/tool calls in correct order)
			self.messages.push(ChatMessage::assistant(content.clone()));
		} else if let Some(calls_ref) = end.captured_tool_calls() {
			// Fallback: build assistant message from tool calls only
			let calls: Vec<ToolCall> = calls_ref.into_iter().cloned().collect();
			if !calls.is_empty() {
				self.messages.push(ChatMessage::from(calls));
			}
		}

		// Append the tool response turn
		self.messages.push(ChatMessage::from(tool_response));
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

	/// Joins all system content with one empty line between entries.
	pub fn join_systems(&self) -> Option<String> {
		let mut systems: Option<String> = None;

		for system in self.iter_systems() {
			let systems_content = systems.get_or_insert_with(String::new);

			support::combine_text_with_empty_line(systems_content, system);
		}

		systems
	}

	#[deprecated(note = "use join_systems()")]
	/// Deprecated alias for [`Self::join_systems`].
	pub fn combine_systems(&self) -> Option<String> {
		self.join_systems()
	}
}

impl From<Vec<ChatMessage>> for ChatRequest {
	fn from(messages: Vec<ChatMessage>) -> Self {
		Self {
			system: None,
			messages,
			tools: None,
			previous_response_id: None,
			store: None,
		}
	}
}

// endregion: --- ChatRequest
