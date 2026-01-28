//! Response types for Anthropic's web search tool results.
//!
//! When using the web_search tool, the response may contain:
//! - `ServerToolUse`: The model's invocation of the web search tool
//! - `WebSearchToolResult`: The results returned by the web search
//! - `TextWithCitations`: Text content that includes inline citations

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A citation referencing a web search result.
///
/// Citations link text content back to the source documents
/// from which the information was derived.
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
}

impl Citation {
	/// Returns an approximate in-memory size of this `Citation`, in bytes.
	pub fn size(&self) -> usize {
		self.url.len()
			+ self.title.len()
			+ self.cited_text.as_ref().map(|s| s.len()).unwrap_or(0)
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
