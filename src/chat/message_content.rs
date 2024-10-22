use crate::chat::{ToolCall, ToolResponse};
use derive_more::derive::From;
use serde::{Deserialize, Serialize};

/// Currently, it only supports Text,
/// but the goal is to support multi-part message content (see below)
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum MessageContent {
	/// Text content
	Text(String),

	/// Tool calls
	#[from]
	ToolCalls(Vec<ToolCall>),

	/// Tool call Responses
	#[from]
	ToolResponses(Vec<ToolResponse>),
}

/// Constructors
impl MessageContent {
	/// Create a new MessageContent with the Text variant
	pub fn from_text(content: impl Into<String>) -> Self {
		MessageContent::Text(content.into())
	}

	/// Create a new MessageContent with the ToolCalls variant
	pub fn from_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
		MessageContent::ToolCalls(tool_calls)
	}
}

/// Getters
impl MessageContent {
	/// Returns the MessageContent as &str, only if it is MessageContent::Text
	/// Otherwise, it returns None.
	/// NOTE: As of now, it always returns Some(..) because MessageContent has only the Text variant.
	///       However, this is in preparation for future expansions.
	pub fn text_as_str(&self) -> Option<&str> {
		match self {
			MessageContent::Text(content) => Some(content.as_str()),
			MessageContent::ToolCalls(_) => None,
			MessageContent::ToolResponses(_) => None,
		}
	}

	/// Consumes the MessageContent and returns it as &str,
	/// only if it is MessageContent::Text; otherwise, it returns None.
	///
	/// NOTE: As of now, it always returns Some(..) because MessageContent has only the Text variant.
	///       However, this is in preparation for future expansions.
	pub fn text_into_string(self) -> Option<String> {
		match self {
			MessageContent::Text(content) => Some(content),
			MessageContent::ToolCalls(_) => None,
			MessageContent::ToolResponses(_) => None,
		}
	}

	/// Checks if the text content or the tools calls is empty.
	pub fn is_empty(&self) -> bool {
		match self {
			MessageContent::Text(content) => content.is_empty(),
			MessageContent::ToolCalls(tool_calls) => tool_calls.is_empty(),
			MessageContent::ToolResponses(tool_responses) => tool_responses.is_empty(),
		}
	}
}

// region:    --- Froms

impl From<String> for MessageContent {
	fn from(s: String) -> Self {
		MessageContent::from_text(s)
	}
}

impl<'a> From<&'a str> for MessageContent {
	fn from(s: &'a str) -> Self {
		MessageContent::from_text(s.to_string())
	}
}

impl From<&String> for MessageContent {
	fn from(s: &String) -> Self {
		MessageContent::from_text(s.clone())
	}
}

impl From<ToolResponse> for MessageContent {
	fn from(tool_response: ToolResponse) -> Self {
		MessageContent::ToolResponses(vec![tool_response])
	}
}

// endregion: --- Froms

// NOTE: The goal is to add a Parts variant with ContentPart for multipart support
//
// ````
// pub enum MessageContent {
// 	Text(String),
//  Parts(Vec<ContentPart>)` variant to `MessageContent`
// }
// ```
//
// With something like this:
// ```
// pub enum ContentPart {
// 	Text(String),
// 	Image(ImagePart)
// }
//
// pub enum ImagePart {
// 	Local(PathBuf),
// 	Remote(Url),
// 	Base64(String)
// }
// ```
