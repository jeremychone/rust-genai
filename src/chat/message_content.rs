use crate::chat::{ToolCall, ToolResponse};
use derive_more::derive::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum MessageContent {
	/// Text content
	Text(String),

	/// Content parts
	Parts(Vec<ContentPart>),

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

	/// Create a new MessageContent from provided content parts
	pub fn from_parts(parts: impl Into<Vec<ContentPart>>) -> Self {
		MessageContent::Parts(parts.into())
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
	///
	/// NOTE: When multi parts content, this will return None and won't concatenate the text parts.
	pub fn text_as_str(&self) -> Option<&str> {
		match self {
			MessageContent::Text(content) => Some(content.as_str()),
			MessageContent::Parts(_) => None,
			MessageContent::ToolCalls(_) => None,
			MessageContent::ToolResponses(_) => None,
		}
	}

	/// Consumes the MessageContent and returns it as &str,
	/// only if it is MessageContent::Text; otherwise, it returns None.
	///
	/// NOTE: When multi parts content, this will return None and won't concatenate the text parts.
	pub fn text_into_string(self) -> Option<String> {
		match self {
			MessageContent::Text(content) => Some(content),
			MessageContent::Parts(_) => None,
			MessageContent::ToolCalls(_) => None,
			MessageContent::ToolResponses(_) => None,
		}
	}

	/// Checks if the text content or the tools calls is empty.
	pub fn is_empty(&self) -> bool {
		match self {
			MessageContent::Text(content) => content.is_empty(),
			MessageContent::Parts(parts) => parts.is_empty(),
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

impl From<Vec<ContentPart>> for MessageContent {
	fn from(parts: Vec<ContentPart>) -> Self {
		MessageContent::Parts(parts)
	}
}

// endregion: --- Froms

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum ContentPart {
	Text(String),
	Image {
		content: String,
		content_type: String,
		source: ImageSource,
	},
}

/// Constructors
impl ContentPart {
	pub fn from_text(text: impl Into<String>) -> ContentPart {
		ContentPart::Text(text.into())
	}

	pub fn from_image_b64(content_type: impl Into<String>, content: impl Into<String>) -> ContentPart {
		ContentPart::Image {
			content: content.into(),
			content_type: content_type.into(),
			source: ImageSource::Base64,
		}
	}

	pub fn from_image_url(content_type: impl Into<String>, url: impl Into<String>) -> ContentPart {
		ContentPart::Image {
			content: url.into(),
			content_type: content_type.into(),
			source: ImageSource::Url,
		}
	}
}

// region:    --- Froms

impl<'a> From<&'a str> for ContentPart {
	fn from(s: &'a str) -> Self {
		ContentPart::Text(s.to_string())
	}
}

// endregion: --- Froms

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum ImageSource {
	Url,
	Base64, // No `Local` location, this would require handling errors like "file not found" etc.
	        // Such file can be easily provided by user as Base64, also can implement convenient
	        // TryFrom<File> to Base64 version. All LLMs accepts local Images only as Base64
}
