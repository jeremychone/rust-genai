//! OAuth credentials for Claude Code CLI authentication.
//!
//! This module provides the `OAuthCredentials` struct for handling OAuth tokens
//! used with Claude Code CLI (tokens starting with `sk-ant-oat-`).

use super::oauth_config::OAuthConfig;

/// OAuth credentials for Claude Code CLI authentication.
///
/// These credentials are used when authenticating with OAuth tokens
/// instead of API keys. The main difference from API key authentication:
/// - Uses `Authorization: Bearer {token}` instead of `x-api-key`
/// - Requires specific anthropic-beta headers
/// - Needs tool name prefixing and system prompt injection
#[derive(Clone)]
pub struct OAuthCredentials {
	/// The OAuth access token (e.g., `sk-ant-oat-...`)
	pub access_token: String,

	/// Optional refresh token for token renewal.
	pub refresh_token: Option<String>,

	/// Optional expiration timestamp (Unix epoch seconds)
	pub expires_at: Option<u64>,

	/// OAuth configuration for request transformations.
	/// Controls which workarounds are applied (system prompt, tool prefixing, etc.)
	pub oauth_config: OAuthConfig,
}

/// Constructors
impl OAuthCredentials {
	/// Create new OAuth credentials with just an access token.
	pub fn new(access_token: impl Into<String>) -> Self {
		Self {
			access_token: access_token.into(),
			refresh_token: None,
			expires_at: None,
			oauth_config: OAuthConfig::default(),
		}
	}

	/// Add a refresh token to the credentials.
	pub fn with_refresh_token(mut self, token: impl Into<String>) -> Self {
		self.refresh_token = Some(token.into());
		self
	}

	/// Set the expiration timestamp.
	pub fn with_expires_at(mut self, expires_at: u64) -> Self {
		self.expires_at = Some(expires_at);
		self
	}

	/// Set the OAuth configuration for request transformations.
	///
	/// # Example
	/// ```
	/// use genai::resolver::{OAuthCredentials, OAuthConfig};
	///
	/// let creds = OAuthCredentials::new("sk-ant-oat-...")
	///     .with_oauth_config(OAuthConfig::full_cloaking());
	/// ```
	pub fn with_oauth_config(mut self, config: OAuthConfig) -> Self {
		self.oauth_config = config;
		self
	}
}

/// Token validation and utilities
impl OAuthCredentials {
	/// Check if the token has expired (with 5 minute buffer).
	///
	/// Returns `false` if no expiration time is set.
	/// Returns `true` (fail-safe) if system time cannot be determined.
	pub fn is_expired(&self) -> bool {
		const BUFFER_SECONDS: u64 = 5 * 60; // 5 minutes

		match self.expires_at {
			Some(expires_at) => {
				// Fail-safe: if time cannot be determined, consider token expired
				let now = std::time::SystemTime::now()
					.duration_since(std::time::UNIX_EPOCH)
					.map(|d| d.as_secs())
					.unwrap_or(u64::MAX);

				now + BUFFER_SECONDS >= expires_at
			}
			None => false,
		}
	}

	/// Check if a token string is an OAuth token.
	///
	/// OAuth tokens from Claude Code CLI start with `sk-ant-oat` (e.g., `sk-ant-oat01-...`).
	pub fn is_oauth_token(token: &str) -> bool {
		token.starts_with("sk-ant-oat")
	}
}

// region:    --- OAuthCredentials Std Impls

impl std::fmt::Debug for OAuthCredentials {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OAuthCredentials")
			.field("access_token", &"REDACTED")
			.field("refresh_token", &self.refresh_token.as_ref().map(|_| "REDACTED"))
			.field("expires_at", &self.expires_at)
			.field("oauth_config", &self.oauth_config)
			.finish()
	}
}

// endregion: --- OAuthCredentials Std Impls

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_is_oauth_token() {
		assert!(OAuthCredentials::is_oauth_token("sk-ant-oat-abc123"));
		assert!(OAuthCredentials::is_oauth_token("sk-ant-oat-SFMyNTY"));
		assert!(OAuthCredentials::is_oauth_token("sk-ant-oat01-abc123")); // New format
		// Should NOT match tokens that don't start with the prefix
		assert!(!OAuthCredentials::is_oauth_token("prefix-sk-ant-oat-abc123"));
		assert!(!OAuthCredentials::is_oauth_token("sk-ant-api01-abc123"));
		assert!(!OAuthCredentials::is_oauth_token("regular-api-key"));
	}

	#[test]
	fn test_not_expired_when_no_expiry() {
		let creds = OAuthCredentials::new("sk-ant-oat-test");
		assert!(!creds.is_expired());
	}

	#[test]
	fn test_expired_token() {
		let creds = OAuthCredentials::new("sk-ant-oat-test").with_expires_at(0); // Unix epoch = definitely expired
		assert!(creds.is_expired());
	}

	#[test]
	fn test_builder_pattern() {
		let creds = OAuthCredentials::new("sk-ant-oat-test")
			.with_refresh_token("refresh-token")
			.with_expires_at(9999999999);

		assert_eq!(creds.access_token, "sk-ant-oat-test");
		assert_eq!(creds.refresh_token, Some("refresh-token".to_string()));
		assert_eq!(creds.expires_at, Some(9999999999));
	}
}
