use serde::{Deserialize, Serialize};

/// Currently, it only supports Text,
/// but the goal is to support multi-part message content (see below)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
	/// Text content
	Text(String),
}

/// Constructors
impl MessageContent {
	/// Create a new MessageContent with the Text variant
	pub fn text(content: impl Into<String>) -> Self {
		MessageContent::Text(content.into())
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
		}
	}

	/// Checks if the text content is empty (for now)
	/// Later, this will also validate each variant to check if they can be considered "empty"
	pub fn is_empty(&self) -> bool {
		match self {
			MessageContent::Text(content) => content.is_empty(),
		}
	}
}

// region:    --- Froms

/// Blanket implementation for MessageContent::Text for anything that implements Into<String>
/// Note: This means that when we support base64 as images, it should not use `.into()` for MessageContent.
///       It should be acceptable but may need reassessment.
impl<T> From<T> for MessageContent
where
	T: Into<String>,
{
	fn from(s: T) -> Self {
		MessageContent::text(s)
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
