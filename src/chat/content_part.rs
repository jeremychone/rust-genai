use crate::chat::{ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
	pub fn from_text(text: impl Into<String>) -> ContentPart {
		ContentPart::Text(text.into())
	}

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
	pub fn as_text(&self) -> Option<&str> {
		if let ContentPart::Text(content) = self {
			Some(content.as_str())
		} else {
			None
		}
	}

	pub fn into_text(self) -> Option<String> {
		if let ContentPart::Text(content) = self {
			Some(content)
		} else {
			None
		}
	}

	pub fn as_tool_call(&self) -> Option<&ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	pub fn into_tool_call(self) -> Option<ToolCall> {
		if let ContentPart::ToolCall(tool_call) = self {
			Some(tool_call)
		} else {
			None
		}
	}

	pub fn as_tool_response(&self) -> Option<&ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	pub fn into_tool_response(self) -> Option<ToolResponse> {
		if let ContentPart::ToolResponse(tool_response) = self {
			Some(tool_response)
		} else {
			None
		}
	}

	pub fn as_binary(&self) -> Option<&Binary> {
		if let ContentPart::Binary(binary) = self {
			Some(binary)
		} else {
			None
		}
	}

	// TODO: into_binary when we have the Binary type
}

/// is_.. Accessors
impl ContentPart {
	#[allow(unused)]
	pub fn is_text(&self) -> bool {
		matches!(self, ContentPart::Text(_))
	}
	pub fn is_image(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim_start().to_ascii_lowercase().starts_with("image/"),
			_ => false,
		}
	}

	#[allow(unused)]
	pub fn is_pdf(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim_start().eq_ignore_ascii_case("application/pdf"),
			_ => false,
		}
	}

	pub fn is_tool_call(&self) -> bool {
		matches!(self, ContentPart::ToolCall(_))
	}

	pub fn is_tool_response(&self) -> bool {
		matches!(self, ContentPart::ToolResponse(_))
	}
}

// endregion: --- Content Part

// region:    --- Binary

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binary {
	pub content_type: String,
	pub source: BinarySource,
	pub name: Option<String>,
}

impl Binary {
	pub fn new(content_type: impl Into<String>, source: BinarySource, name: Option<String>) -> Self {
		Self {
			name,
			content_type: content_type.into(),
			source,
		}
	}
}

impl Binary {
	pub fn is_image(&self) -> bool {
		self.content_type.trim_start().to_ascii_lowercase().starts_with("image/")
	}

	pub fn is_pdf(&self) -> bool {
		self.content_type.trim_start().eq_ignore_ascii_case("application/pdf")
	}
}

// endregion: --- Binary

// region:    --- BinarySource

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
