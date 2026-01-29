//! Response types for Anthropic's server tool results (web search, web fetch).
//!
//! When using the web_search tool, the response may contain:
//! - `ServerToolUse`: The model's invocation of the web search tool
//! - `WebSearchToolResult`: The results returned by the web search
//! - `TextWithCitations`: Text content that includes inline citations
//!
//! When using the web_fetch tool, the response may contain:
//! - `ServerToolUse`: The model's invocation of the web fetch tool
//! - `WebFetchToolResult`: The fetched content from a URL

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A citation referencing a web search or web fetch result.
///
/// Citations link text content back to the source documents
/// from which the information was derived.
///
/// For web search citations (type `web_search_result_location`):
/// - `url`, `title`, `cited_text`, `encrypted_index` are populated
///
/// For web fetch citations (type `char_location`):
/// - `url`, `title`, `cited_text`, `document_index`, `start_char_index`, `end_char_index` are populated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
	/// URL of the source document.
	pub url: String,

	/// Title of the source document.
	pub title: String,

	/// The specific text that was cited from the source.
	/// May be None if the citation is for the source as a whole.
	pub cited_text: Option<String>,

	/// Start index in the text where this citation applies.
	pub start_index: Option<u32>,

	/// End index in the text where this citation applies.
	pub end_index: Option<u32>,

	/// Encrypted index for web search citations (multi-turn support).
	pub encrypted_index: Option<String>,

	/// Document index for web fetch citations (char_location type).
	pub document_index: Option<u32>,

	/// Start character index in the fetched document (char_location type).
	pub start_char_index: Option<u32>,

	/// End character index in the fetched document (char_location type).
	pub end_char_index: Option<u32>,
}

impl Citation {
	/// Returns an approximate in-memory size of this `Citation`, in bytes.
	pub fn size(&self) -> usize {
		self.url.len()
			+ self.title.len()
			+ self.cited_text.as_ref().map(|s| s.len()).unwrap_or(0)
			+ self.encrypted_index.as_ref().map(|s| s.len()).unwrap_or(0)
	}
}

/// Text content with inline citations from web search.
///
/// This represents a text block where the model has incorporated
/// information from web search results and provided citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextWithCitations {
	/// The text content.
	pub text: String,

	/// Citations for sources referenced in this text.
	pub citations: Vec<Citation>,
}

impl TextWithCitations {
	/// Returns an approximate in-memory size of this `TextWithCitations`, in bytes.
	pub fn size(&self) -> usize {
		self.text.len() + self.citations.iter().map(|c| c.size()).sum::<usize>()
	}
}

/// A single web search result item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResultItem {
	/// URL of the search result.
	pub url: String,

	/// Title of the search result.
	pub title: String,

	/// Age of the page (e.g., "2 days ago").
	pub page_age: Option<String>,

	/// Encrypted content for multi-turn conversations.
	/// This allows Claude to reference the content in follow-up turns.
	pub encrypted_content: Option<String>,
}

impl WebSearchResultItem {
	/// Returns an approximate in-memory size of this `WebSearchResultItem`, in bytes.
	pub fn size(&self) -> usize {
		self.url.len()
			+ self.title.len()
			+ self.page_age.as_ref().map(|s| s.len()).unwrap_or(0)
			+ self.encrypted_content.as_ref().map(|s| s.len()).unwrap_or(0)
	}
}

/// Server-initiated tool use (e.g., web search query).
///
/// This represents the model's invocation of a server-side tool
/// like web_search. The actual execution happens on Anthropic's servers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUse {
	/// Unique identifier for this tool use.
	pub id: String,

	/// Name of the tool being used (e.g., "web_search").
	pub name: String,

	/// Input parameters for the tool (e.g., search query).
	pub input: Value,
}

impl ServerToolUse {
	/// Returns an approximate in-memory size of this `ServerToolUse`, in bytes.
	pub fn size(&self) -> usize {
		self.id.len()
			+ self.name.len()
			+ serde_json::to_string(&self.input)
				.map(|s| s.len())
				.unwrap_or(0)
	}
}

/// Results from a web search tool execution.
///
/// This contains the search results returned by Anthropic's
/// web search infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchToolResult {
	/// The tool_use_id this result corresponds to.
	pub tool_use_id: String,

	/// The search results.
	pub results: Vec<WebSearchResultItem>,
}

impl WebSearchToolResult {
	/// Returns an approximate in-memory size of this `WebSearchToolResult`, in bytes.
	pub fn size(&self) -> usize {
		self.tool_use_id.len() + self.results.iter().map(|r| r.size()).sum::<usize>()
	}
}

// region:    --- Web Fetch Types

/// Results from a web fetch tool execution.
///
/// This contains the fetched content returned by Anthropic's
/// web fetch infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchToolResult {
	/// The tool_use_id this result corresponds to.
	pub tool_use_id: String,

	/// The URL that was fetched (if provided in the response).
	/// Note: The URL may not be present in the response; check ServerToolUse input for the original URL.
	pub url: Option<String>,

	/// The fetched content.
	pub content: WebFetchContent,

	/// ISO 8601 timestamp of when the content was retrieved.
	pub retrieved_at: Option<String>,
}

impl WebFetchToolResult {
	/// Returns an approximate in-memory size of this `WebFetchToolResult`, in bytes.
	pub fn size(&self) -> usize {
		self.tool_use_id.len()
			+ self.url.as_ref().map(|s| s.len()).unwrap_or(0)
			+ self.content.size()
			+ self.retrieved_at.as_ref().map(|s| s.len()).unwrap_or(0)
	}
}

/// Content fetched from a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchContent {
	/// The document type (e.g., "document").
	pub document_type: String,

	/// The source type (e.g., "text" or "base64").
	pub source_type: String,

	/// The media type (e.g., "text/plain", "application/pdf").
	pub media_type: String,

	/// The actual content data (text or base64-encoded).
	pub data: String,

	/// Optional title extracted from the document.
	pub title: Option<String>,
}

impl WebFetchContent {
	/// Returns an approximate in-memory size of this `WebFetchContent`, in bytes.
	pub fn size(&self) -> usize {
		self.document_type.len()
			+ self.source_type.len()
			+ self.media_type.len()
			+ self.data.len()
			+ self.title.as_ref().map(|s| s.len()).unwrap_or(0)
	}
}

// endregion: --- Web Fetch Types

// region:    --- Error Types

/// Error from a server tool execution (web search or web fetch).
///
/// Returned when a server tool fails (e.g., rate limit, invalid URL, blocked domain).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolError {
	/// The tool_use_id this error corresponds to.
	pub tool_use_id: String,

	/// Name of the tool that failed (e.g., "web_search", "web_fetch").
	pub tool_name: String,

	/// The error code (e.g., "max_searches_reached", "fetch_failed", "domain_not_allowed").
	pub error_code: String,
}

// endregion: --- Error Types
