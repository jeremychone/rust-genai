use crate::chat::{MessageContent, ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// A single chat message (system, user, assistant, or tool).
///
/// Design:
/// - Uses one struct with a role field instead of role-specific enum variants.
/// - Payload lives in MessageContent; ChatRole distinguishes the role.
/// - MessageContent is a multipart format, with `Vec<ContentPart>`
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The message role.
	pub role: ChatRole,

	/// Message content.
	pub content: MessageContent,

	/// Optional per-message options (e.g., cache control).
	pub options: Option<MessageOptions>,
}

/// Constructors
impl ChatMessage {
	/// Constructs a system message.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs an assistant message.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a user message.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			options: None,
		}
	}
}

impl ChatMessage {
	/// Attaches options to this message.
	pub fn with_options(mut self, options: impl Into<MessageOptions>) -> Self {
		self.options = Some(options.into());
		self
	}
}
// region:    --- MessageOptions

#[derive(Debug, Clone, Serialize, Deserialize, From)]
/// Per-message options (e.g., cache control).
pub struct MessageOptions {
	#[from]
	pub cache_control: Option<CacheControl>,
}

/// Cache control
///
/// Notes:
/// - Currently used for Anthropic only.
/// - Anthropic applies cache_control at the content-part level; genai exposes it at the
///   ChatMessage level and maps it appropriately.
/// - OpenAI ignores it; Gemini uses a separate API, so it is not supported there yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheControl {
	Ephemeral,
}

impl From<CacheControl> for MessageOptions {
	fn from(cache_control: CacheControl) -> Self {
		Self {
			cache_control: Some(cache_control),
		}
	}
}
// endregion: --- MessageOptions

/// Chat roles.
#[derive(Debug, Clone, Serialize, Deserialize, derive_more::Display)]
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
			options: None,
		}
	}
}

impl From<ToolResponse> for ChatMessage {
	fn from(value: ToolResponse) -> Self {
		Self {
			role: ChatRole::Tool,
			content: MessageContent::from(value),
			options: None,
		}
	}
}

// endregion: --- Froms
