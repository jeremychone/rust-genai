//! Web search configuration types for Anthropic's built-in web search tool.

use serde::{Deserialize, Serialize};

/// Configuration for Anthropic's web_search tool.
///
/// This allows customizing the behavior of the built-in web search tool,
/// including domain filtering, usage limits, and user location for localized results.
///
/// # Example
/// ```rust
/// use genai::chat::WebSearchConfig;
///
/// let config = WebSearchConfig::default()
///     .with_max_uses(5)
///     .with_allowed_domains(vec!["rust-lang.org".into(), "docs.rs".into()]);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSearchConfig {
	/// Maximum number of web searches allowed per request.
	/// If not set, the model decides how many searches to perform.
	pub max_uses: Option<u32>,

	/// Only search results from these domains will be included.
	/// Cannot be used together with `blocked_domains`.
	pub allowed_domains: Option<Vec<String>>,

	/// Search results from these domains will be excluded.
	/// Cannot be used together with `allowed_domains`.
	pub blocked_domains: Option<Vec<String>>,

	/// Approximate user location for localized search results.
	pub user_location: Option<UserLocation>,
}

impl WebSearchConfig {
	/// Set the maximum number of web searches allowed.
	pub fn with_max_uses(mut self, max_uses: u32) -> Self {
		self.max_uses = Some(max_uses);
		self
	}

	/// Set allowed domains (allowlist).
	/// Only results from these domains will be included.
	pub fn with_allowed_domains(mut self, domains: Vec<String>) -> Self {
		self.allowed_domains = Some(domains);
		self
	}

	/// Set blocked domains (blocklist).
	/// Results from these domains will be excluded.
	pub fn with_blocked_domains(mut self, domains: Vec<String>) -> Self {
		self.blocked_domains = Some(domains);
		self
	}

	/// Set the user location for localized search results.
	pub fn with_user_location(mut self, location: UserLocation) -> Self {
		self.user_location = Some(location);
		self
	}
}

/// Approximate user location for localized search results.
///
/// All fields are optional. Providing more location details can improve
/// the relevance of localized search results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserLocation {
	/// City name (e.g., "San Francisco")
	pub city: Option<String>,

	/// Region/state (e.g., "California")
	pub region: Option<String>,

	/// Country code or name (e.g., "US" or "United States")
	pub country: Option<String>,

	/// Timezone (e.g., "America/Los_Angeles")
	pub timezone: Option<String>,
}

impl UserLocation {
	/// Create a new UserLocation with all fields.
	pub fn new(
		city: impl Into<Option<String>>,
		region: impl Into<Option<String>>,
		country: impl Into<Option<String>>,
		timezone: impl Into<Option<String>>,
	) -> Self {
		Self {
			city: city.into(),
			region: region.into(),
			country: country.into(),
			timezone: timezone.into(),
		}
	}

	/// Set the city.
	pub fn with_city(mut self, city: impl Into<String>) -> Self {
		self.city = Some(city.into());
		self
	}

	/// Set the region/state.
	pub fn with_region(mut self, region: impl Into<String>) -> Self {
		self.region = Some(region.into());
		self
	}

	/// Set the country.
	pub fn with_country(mut self, country: impl Into<String>) -> Self {
		self.country = Some(country.into());
		self
	}

	/// Set the timezone.
	pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
		self.timezone = Some(timezone.into());
		self
	}
}
