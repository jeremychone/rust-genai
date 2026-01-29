//! Web fetch configuration types for Anthropic's built-in web fetch tool.

use serde::{Deserialize, Serialize};

/// Configuration for Anthropic's web_fetch tool.
///
/// This allows customizing the behavior of the built-in web fetch tool,
/// including domain filtering, usage limits, and content settings.
///
/// # Example
/// ```rust
/// use genai::chat::WebFetchConfig;
///
/// let config = WebFetchConfig::default()
///     .with_max_uses(5)
///     .with_allowed_domains(vec!["rust-lang.org".into(), "docs.rs".into()]);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebFetchConfig {
	/// Maximum number of web fetches allowed per request.
	/// If not set, the model decides how many fetches to perform.
	pub max_uses: Option<u32>,

	/// Only URLs from these domains will be fetched.
	/// Cannot be used together with `blocked_domains`.
	pub allowed_domains: Option<Vec<String>>,

	/// URLs from these domains will not be fetched.
	/// Cannot be used together with `allowed_domains`.
	pub blocked_domains: Option<Vec<String>>,

	/// Citation configuration for the web fetch tool.
	pub citations: Option<WebFetchCitations>,

	/// Maximum number of content tokens to return per fetch.
	/// Limits the size of fetched content.
	pub max_content_tokens: Option<u32>,
}

impl WebFetchConfig {
	/// Set the maximum number of web fetches allowed.
	pub fn with_max_uses(mut self, max_uses: u32) -> Self {
		self.max_uses = Some(max_uses);
		self
	}

	/// Set allowed domains (allowlist).
	/// Only URLs from these domains will be fetched.
	pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
		self.allowed_domains = Some(domains);
		self
	}

	/// Set blocked domains (blocklist).
	/// URLs from these domains will not be fetched.
	pub fn with_blocked_domains(mut self, domains: Vec<String>) -> Self {
		self.blocked_domains = Some(domains);
		self
	}

	/// Set citation configuration.
	pub fn with_citations(mut self, citations: WebFetchCitations) -> Self {
		self.citations = Some(citations);
		self
	}

	/// Enable citations with default settings.
	pub fn with_citations_enabled(mut self) -> Self {
		self.citations = Some(WebFetchCitations { enabled: true });
		self
	}

	/// Set the maximum number of content tokens per fetch.
	pub fn with_max_content_tokens(mut self, max_tokens: u32) -> Self {
		self.max_content_tokens = Some(max_tokens);
		self
	}
}

/// Citation configuration for web fetch results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebFetchCitations {
	/// Whether citations are enabled for fetched content.
	pub enabled: bool,
}

impl WebFetchCitations {
	/// Create a new citations config with citations enabled.
	pub fn enabled() -> Self {
		Self { enabled: true }
	}

	/// Create a new citations config with citations disabled.
	pub fn disabled() -> Self {
		Self { enabled: false }
	}
}
