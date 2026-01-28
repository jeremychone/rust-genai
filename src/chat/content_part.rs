use crate::Result;
use crate::chat::{Binary, ServerToolUse, TextWithCitations, ToolCall, ToolResponse, WebFetchToolResult, WebSearchToolResult};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A single content segment in a chat message.
///
/// Variants cover plain text, binary payloads (e.g., images/PDF), tool calls/responses,
/// and web search related content.
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

	#[from(ignore)]
	ThoughtSignature(String),

	// -- Web Search / Web Fetch variants (Anthropic)

	/// Text content with inline citations from web search/fetch.
	#[from]
	TextWithCitations(TextWithCitations),

	/// Server-initiated tool use (e.g., web search query, web fetch).
	#[from]
	ServerToolUse(ServerToolUse),

	/// Results from a web search tool execution.
	#[from]
	WebSearchToolResult(WebSearchToolResult),

	/// Results from a web fetch tool execution.
	#[from]
	WebFetchToolResult(WebFetchToolResult),
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
		ContentPart::Binary(Binary::from_base64(content_type, content, name))
	}

	/// Create a binary content part referencing a URL.
	///
	/// Note: Only some providers accept URL-based inputs.
	pub fn from_binary_url(
		content_type: impl Into<String>,
		url: impl Into<String>,
		name: Option<String>,
	) -> ContentPart {
		ContentPart::Binary(Binary::from_url(content_type, url, name))
	}

	/// Create a binary content part from a file path.
	///
	/// Reads the file, determines the MIME type from the file extension,
	/// and base64-encodes the content.
	///
	/// - file_path: Path to the file to read.
	///
	/// Returns an error if the file cannot be read.
	pub fn from_binary_file(file_path: impl AsRef<Path>) -> Result<ContentPart> {
		Ok(ContentPart::Binary(Binary::from_file(file_path)?))
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

	/// Borrow the thought signature if present.
	pub fn as_thought_signature(&self) -> Option<&str> {
		if let ContentPart::ThoughtSignature(thought_signature) = self {
			Some(thought_signature)
		} else {
			None
		}
	}

	/// Extract the thought, consuming the part.
	pub fn into_thought_signature(self) -> Option<String> {
		if let ContentPart::ThoughtSignature(thought_signature) = self {
			Some(thought_signature)
		} else {
			None
		}
	}

	/// Borrow the text with citations if present.
	pub fn as_text_with_citations(&self) -> Option<&TextWithCitations> {
		if let ContentPart::TextWithCitations(twc) = self {
			Some(twc)
		} else {
			None
		}
	}

	/// Extract the text with citations, consuming the part.
	pub fn into_text_with_citations(self) -> Option<TextWithCitations> {
		if let ContentPart::TextWithCitations(twc) = self {
			Some(twc)
		} else {
			None
		}
	}

	/// Borrow the server tool use if present.
	pub fn as_server_tool_use(&self) -> Option<&ServerToolUse> {
		if let ContentPart::ServerToolUse(stu) = self {
			Some(stu)
		} else {
			None
		}
	}

	/// Extract the server tool use, consuming the part.
	pub fn into_server_tool_use(self) -> Option<ServerToolUse> {
		if let ContentPart::ServerToolUse(stu) = self {
			Some(stu)
		} else {
			None
		}
	}

	/// Borrow the web search tool result if present.
	pub fn as_web_search_tool_result(&self) -> Option<&WebSearchToolResult> {
		if let ContentPart::WebSearchToolResult(wsr) = self {
			Some(wsr)
		} else {
			None
		}
	}

	/// Extract the web search tool result, consuming the part.
	pub fn into_web_search_tool_result(self) -> Option<WebSearchToolResult> {
		if let ContentPart::WebSearchToolResult(wsr) = self {
			Some(wsr)
		} else {
			None
		}
	}

	/// Borrow the web fetch tool result if present.
	pub fn as_web_fetch_tool_result(&self) -> Option<&WebFetchToolResult> {
		if let ContentPart::WebFetchToolResult(wfr) = self {
			Some(wfr)
		} else {
			None
		}
	}

	/// Extract the web fetch tool result, consuming the part.
	pub fn into_web_fetch_tool_result(self) -> Option<WebFetchToolResult> {
		if let ContentPart::WebFetchToolResult(wfr) = self {
			Some(wfr)
		} else {
			None
		}
	}
}

/// Computed accessors
impl ContentPart {
	/// Returns an approximate in-memory size of this `ContentPart`, in bytes.
	///
	/// - For `Text` and `ThoughtSignature`: the UTF-8 length of the string.
	/// - For `Binary`: delegates to `Binary::size()`.
	/// - For `ToolCall`: delegates to `ToolCall::size()`.
	/// - For `ToolResponse`: delegates to `ToolResponse::size()`.
	/// - For web search types: delegates to their respective `size()` methods.
	pub fn size(&self) -> usize {
		match self {
			ContentPart::Text(text) => text.len(),
			ContentPart::Binary(binary) => binary.size(),
			ContentPart::ToolCall(tool_call) => tool_call.size(),
			ContentPart::ToolResponse(tool_response) => tool_response.size(),
			ContentPart::ThoughtSignature(thought) => thought.len(),
			ContentPart::TextWithCitations(twc) => twc.size(),
			ContentPart::ServerToolUse(stu) => stu.size(),
			ContentPart::WebSearchToolResult(wsr) => wsr.size(),
			ContentPart::WebFetchToolResult(wfr) => wfr.size(),
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

	/// Returns true if this part is a binary audio (content_type starts with "audio/").
	pub fn is_audio(&self) -> bool {
		match self {
			ContentPart::Binary(binary) => binary.content_type.trim().to_ascii_lowercase().starts_with("audio/"),
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

	/// Returns true if this part is a thought.
	pub fn is_thought_signature(&self) -> bool {
		matches!(self, ContentPart::ThoughtSignature(_))
	}

	/// Returns true if this part contains text with citations.
	pub fn is_text_with_citations(&self) -> bool {
		matches!(self, ContentPart::TextWithCitations(_))
	}

	/// Returns true if this part contains a server tool use.
	pub fn is_server_tool_use(&self) -> bool {
		matches!(self, ContentPart::ServerToolUse(_))
	}

	/// Returns true if this part contains web search tool results.
	pub fn is_web_search_tool_result(&self) -> bool {
		matches!(self, ContentPart::WebSearchToolResult(_))
	}

	/// Returns true if this part contains web fetch tool results.
	pub fn is_web_fetch_tool_result(&self) -> bool {
		matches!(self, ContentPart::WebFetchToolResult(_))
	}
}
