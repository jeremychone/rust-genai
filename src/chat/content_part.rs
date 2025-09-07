use crate::chat::{ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// region:    --- Content Part

/// A single content segment in a chat message.
///
/// Variants cover plain text, binary payloads (e.g., images/PDF), and tool calls/responses.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum ContentPart {
	#[from(String, &String, &str)]
	Text(String),

	#[from]
	Binary(Binary),

	#[from]
	ToolCall(ToolCall),

	#[from]
	ToolResponse(ToolResponse),
}

/// Constructors
impl ContentPart {
	/// Create a text content part.
	pub fn from_text(text: impl Into<String>) -> ContentPart {
		ContentPart::Text(text.into())
	}

	/// Create a binary content part from a base64 payload.
	///
	/// - content_type: MIME type (e.g., "image/png", "application/pdf").
	/// - content: base64-encoded bytes.
	/// - name: optional display name or filename.
	pub fn from_binary_base64(
		content_type: impl Into<String>,
		content: impl Into<Arc<str>>,
		name: Option<String>,
	) -> ContentPart {
		ContentPart::Binary(Binary {
			name,
			content_type: content_type.into(),
			source: BinarySource::Base64(content.into()),
		})
	}

	/// Create a binary content part referencing a URL.
	///
	/// Note: Only some providers accept URL-based inputs.
	pub fn from_binary_url(
		content_type: impl Into<String>,
		url: impl Into<String>,
		name: Option<String>,
	) -> ContentPart {
		ContentPart::Binary(Binary {
			name,
			content_type: content_type.into(),
			source: BinarySource::Url(url.into()),
		})
	}
}

/// as_.., into_.. Accessors
impl ContentPart {
	/// Borrow the inner text if this part is text.
	pub fn as_text(&self) -> Option<&str> {
		if let ContentPart::Text(content) = self {
			Some(content.as_str())
		} else {
			None
		}
	}

	/// Extract the text, consuming the part.
	pub fn into_text(self) -> Option<String> {
		if let ContentPart::Text(content) = self {
			Some(content)
		} else {
			None
		}
	}

	/// Borrow the tool call if present.
	pub fn as_tool_call(&self) -> Option<&ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	/// Extract the tool call, consuming the part.
	pub fn into_tool_call(self) -> Option<ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	/// Borrow the tool response if present.
	pub fn as_tool_response(&self) -> Option<&ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	/// Extract the tool response, consuming the part.
	pub fn into_tool_response(self) -> Option<ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	/// Borrow the binary payload if present.
	pub fn as_binary(&self) -> Option<&Binary> {
		if let ContentPart::Binary(binary) = self {
			Some(binary)
		} else {
			None
		}
	}

	// into_binary implemented below
	pub fn into_binary(self) -> Option<Binary> {
		if let ContentPart::Binary(binary) = self {
			Some(binary)
		} else {
			None
		}
	}
}

/// is_.. Accessors
impl ContentPart {
	#[allow(unused)]
	/// Returns true if this part is text.
	pub fn is_text(&self) -> bool {
		matches!(self, ContentPart::Text(_))
	}
	/// Returns true if this part is a binary image (content_type starts with "image/").
	pub fn is_image(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().to_ascii_lowercase().starts_with("image/"),
			_ => false,
		}
	}

	#[allow(unused)]
	/// Returns true if this part is a PDF binary (content_type equals "application/pdf").
	pub fn is_pdf(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().eq_ignore_ascii_case("application/pdf"),
			_ => false,
		}
	}

	/// Returns true if this part contains a tool call.
	pub fn is_tool_call(&self) -> bool {
		matches!(self, ContentPart::ToolCall(_))
	}

	/// Returns true if this part contains a tool response.
	pub fn is_tool_response(&self) -> bool {
		matches!(self, ContentPart::ToolResponse(_))
	}
}

// endregion: --- Content Part

// region:    --- Binary

/// Binary payload attached to a message (e.g., image or PDF).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
	/// MIME type, such as "image/png" or "application/pdf".
	pub content_type: String,

	/// Where the bytes come from (base64 or URL).
	pub source: BinarySource,

	/// Optional display name or filename.
	pub name: Option<String>,
}

impl Binary {
	/// Construct a new Binary value.
	pub fn new(content_type: impl Into<String>, source: BinarySource, name: Option<String>) -> Self {
		Self {
			name,
			content_type: content_type.into(),
			source,
		}
	}
}

impl Binary {
	/// Returns true if this binary is an image (content_type starts with "image/").
	pub fn is_image(&self) -> bool {
		self.content_type.trim().to_ascii_lowercase().starts_with("image/")
	}

	/// Returns true if this binary is a PDF (content_type equals "application/pdf").
	pub fn is_pdf(&self) -> bool {
		self.content_type.trim().eq_ignore_ascii_case("application/pdf")
	}
}

// endregion: --- Binary

// region:    --- BinarySource

/// Origin of a binary payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinarySource {
	/// For models/services that support URL as input
	/// NOTE: Few AI services support this.
	Url(String),

	/// The base64 string of the image
	///
	/// NOTE: Here we use an Arc<str> to avoid cloning large amounts of data when cloning a ChatRequest.
	///       The overhead is minimal compared to cloning relatively large data.
	///       The downside is that it will be an Arc even when used only once, but for this particular data type, the net benefit is positive.
	Base64(Arc<str>),
}

// endregion: --- BinarySource

// No `Local` location; this would require handling errors like "file not found" etc.
// Such a file can be easily provided by the user as Base64, and we can implement a convenient
// TryFrom<File> to Base64 version. All LLMs accept local images only as Base64.
