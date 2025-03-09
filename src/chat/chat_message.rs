use crate::chat::{MessageContent, ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// An individual chat message, for System, User, Assistant, Tool, or ToolResponse
///
/// **Note:**
///
/// The current design uses a single ChatMessage type for all Roles as a struct for now,
/// with the content being the MessageContent, and the role distinguished by the `.role` property.
///
/// This differs from another valid approach where ChatMessage would have been an enum
/// with a variant per role, making the content more aligned with the Role, but adding some type redundancy.
///
/// Both approaches have pros and cons. For now, genai has taken the former approach, but we might revisit this in a "major" release.
///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The role of the message.
	pub role: ChatRole,

	/// The content of the message.
	pub content: MessageContent,

	/// For now, just allow CacheControl, but might support more later
	pub options: Option<MessageOptions>,
}

/// Constructors
impl ChatMessage {
	/// Create a new ChatMessage with the role `ChatRole::System`.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			options: None,
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::Assistant`.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			options: None,
		}
	}

	/// Create a new ChatMessage with the role `ChatRole::User`.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			options: None,
		}
	}
}

impl ChatMessage {
	pub fn with_options(mut self, options: impl Into<MessageOptions>) -> Self {
		self.options = Some(options.into());
		self
	}
}
// region:    --- MessageOptions

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct MessageOptions {
	#[from]
	pub cache_control: Option<CacheControl>,
}

/// Cache control
/// Note - For now, only for Anthropic
///        Also Anthropic put the cache_control at the ContentPart level but for now
///        to keep things simpler, the cache_control is at the ChatMessage leve
///        and genera will create the tright thing
/// Note: OpenAI is transparent, and Gemini has a separate call for it (so not supported for now)
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
