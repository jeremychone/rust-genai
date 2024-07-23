/// For now, supports only Text,
/// But the goal is to support multi-part message content (see below)
#[derive(Debug, Clone)]
pub enum MessageContent {
	Text(String),
}

/// Constructors
impl MessageContent {
	pub fn text(content: impl Into<String>) -> Self {
		MessageContent::Text(content.into())
	}
}

/// Getters
impl MessageContent {
	/// Returns the MessageContent as &str, only if MessageContent::Text
	/// Otherwise returns None.
	/// NOTE: Right now, always return Some(..) because MessageContent has only the Text variant.
	///       But this is in preparation of later.
	pub fn text_as_str(&self) -> Option<&str> {
		match self {
			MessageContent::Text(content) => Some(content.as_str()),
		}
	}

	/// Consume the MessageContent, and returns the MessageContent as &str,
	/// only if MessageContent::Text, Otherwise returns None.
	///
	/// NOTE: Right now, always return Some(..) because MessageContent has only the Text variant.
	///       But this is in preparation of later.
	pub fn text_into_string(self) -> Option<String> {
		match self {
			MessageContent::Text(content) => Some(content),
		}
	}

	/// Will check of the text content is string (for now)
	/// Later, this will also validate each variant to check if they can be considered "empty"
	pub fn is_empty(&self) -> bool {
		match self {
			MessageContent::Text(content) => content.is_empty(),
		}
	}
}

// region:    --- Froms

/// Blanket implementation to MessageContent::Text for anything that have Into<String>
/// Note: This means that when we support base64 as images, it should not use `.into()` for MessageContent.
///        It should be okay, but might need reassessment.
impl<T> From<T> for MessageContent
where
	T: Into<String>,
{
	fn from(s: T) -> Self {
		MessageContent::text(s)
	}
}

// endregion: --- Froms

// NOTE: The goal is to add Parts variant with ContentPart for multipart support
//
// ````
// pub enum MessageContent {
// 	Text(String),
//  Parts(Vec<ContentPart>)` variant to `MessageContent`
// }
// ```
//
// With something like that:
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
