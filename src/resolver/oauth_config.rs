//! OAuth configuration for controlling which transformations are applied.
//!
//! This allows fine-grained control over OAuth workarounds, useful for:
//! - Testing which transformations are actually required
//! - Adapting to changes in Anthropic's API requirements
//! - Debugging OAuth issues

use serde::{Deserialize, Serialize};

/// System prompt text for OAuth requests (Claude Code CLI format).
/// This exact text is validated by Anthropic's API for OAuth tokens.
pub const CLAUDE_CODE_SYSTEM_TEXT: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// Configuration for OAuth request transformations.
///
/// Controls which workarounds/transformations are applied to OAuth requests.
/// By default, only `inject_system_prompt` is enabled (the only required transformation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
	/// Inject Claude Code system prompt.
	/// **REQUIRED** - API validates this exact text for OAuth tokens.
	/// Default: true
	pub inject_system_prompt: bool,

	/// Use JSON array format with cache_control for system prompt.
	/// Optional - tested and confirmed not required.
	/// Default: false
	pub use_cache_control: bool,

	/// Prefix tool names with `proxy_`.
	/// Optional - tested and confirmed not required (even for tool use).
	/// Default: false
	pub prefix_tool_names: bool,

	/// Inject fake metadata.user_id.
	/// Optional - tested and confirmed not required.
	/// Default: false
	pub inject_user_id: bool,

	/// Obfuscate sensitive words with zero-width spaces.
	/// Optional - word list is empty by default.
	/// **Do NOT add "claude" or "anthropic"** - breaks OAuth.
	/// Default: false
	pub obfuscate_sensitive_words: bool,

	/// Add X-Stainless headers to mimic Claude CLI.
	/// Optional - tested and confirmed not required.
	/// Default: false
	pub add_stainless_headers: bool,

	/// Custom Claude Code system prompt text.
	/// Default: "You are Claude Code, Anthropic's official CLI for Claude."
	pub system_prompt_text: Option<String>,
}

impl Default for OAuthConfig {
	fn default() -> Self {
		Self {
			// Required - API validates this exact text
			inject_system_prompt: true,

			// Optional - tests confirmed these are not required
			use_cache_control: false,
			prefix_tool_names: false,
			inject_user_id: false,
			obfuscate_sensitive_words: false,
			add_stainless_headers: false,

			// Use default system prompt
			system_prompt_text: None,
		}
	}
}

impl OAuthConfig {
	/// Create a new config with default settings (minimum required).
	/// Alias for [`Self::minimal()`].
	pub fn new() -> Self {
		Self::default()
	}

	/// Minimal config - only the absolutely required transformation.
	///
	/// Only `inject_system_prompt` is enabled. This is the minimum needed
	/// for OAuth tokens to work with Anthropic API.
	pub fn minimal() -> Self {
		Self::default()
	}

	/// CLIProxyAPI-compatible config.
	///
	/// Matches CLIProxyAPI default behavior:
	/// - System prompt injection: enabled
	/// - Fake user ID injection: enabled
	/// - Everything else: disabled
	pub fn cli_proxy_api_compat() -> Self {
		Self {
			inject_system_prompt: true,
			use_cache_control: false,
			prefix_tool_names: false,
			inject_user_id: true,
			obfuscate_sensitive_words: false,
			add_stainless_headers: false,
			system_prompt_text: None,
		}
	}

	/// Full cloaking - all optional transformations enabled.
	///
	/// Enables everything except obfuscation (word list is empty by default).
	pub fn full_cloaking() -> Self {
		Self {
			inject_system_prompt: true,
			use_cache_control: true,
			prefix_tool_names: true,
			inject_user_id: true,
			obfuscate_sensitive_words: false, // Word list is empty by default
			add_stainless_headers: true,
			system_prompt_text: None,
		}
	}

	/// No transformations (for testing).
	/// **Will not work with OAuth tokens** - API requires system prompt.
	pub fn none() -> Self {
		Self {
			inject_system_prompt: false,
			use_cache_control: false,
			prefix_tool_names: false,
			inject_user_id: false,
			obfuscate_sensitive_words: false,
			add_stainless_headers: false,
			system_prompt_text: None,
		}
	}

	/// Builder: enable/disable system prompt injection.
	pub fn with_inject_system_prompt(mut self, enabled: bool) -> Self {
		self.inject_system_prompt = enabled;
		self
	}

	/// Builder: enable/disable cache_control.
	pub fn with_cache_control(mut self, enabled: bool) -> Self {
		self.use_cache_control = enabled;
		self
	}

	/// Builder: enable/disable tool name prefixing.
	pub fn with_prefix_tool_names(mut self, enabled: bool) -> Self {
		self.prefix_tool_names = enabled;
		self
	}

	/// Builder: enable/disable user_id injection.
	pub fn with_inject_user_id(mut self, enabled: bool) -> Self {
		self.inject_user_id = enabled;
		self
	}

	/// Builder: enable/disable sensitive word obfuscation.
	///
	/// Obfuscates words like "proxy", "api_key", etc. with zero-width spaces.
	/// Safe to use - "claude" and "anthropic" are NOT in the default word list
	/// (they would break OAuth system prompt validation).
	pub fn with_obfuscation(mut self, enabled: bool) -> Self {
		self.obfuscate_sensitive_words = enabled;
		self
	}

	/// Builder: enable/disable X-Stainless headers.
	pub fn with_stainless_headers(mut self, enabled: bool) -> Self {
		self.add_stainless_headers = enabled;
		self
	}

	/// Builder: set custom system prompt text.
	pub fn with_system_prompt_text(mut self, text: impl Into<String>) -> Self {
		self.system_prompt_text = Some(text.into());
		self
	}

	/// Get the system prompt text to use.
	pub fn get_system_prompt_text(&self) -> &str {
		self.system_prompt_text.as_deref().unwrap_or(CLAUDE_CODE_SYSTEM_TEXT)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_default_config() {
		let config = OAuthConfig::default();
		// Only system prompt is required
		assert!(config.inject_system_prompt);
		// Everything else is optional and disabled by default
		assert!(!config.use_cache_control);
		assert!(!config.prefix_tool_names);
		assert!(!config.inject_user_id);
		assert!(!config.obfuscate_sensitive_words);
		assert!(!config.add_stainless_headers);
	}

	#[test]
	fn test_full_cloaking() {
		let config = OAuthConfig::full_cloaking();
		assert!(config.inject_system_prompt);
		assert!(config.inject_user_id);
		// Obfuscation is disabled because it breaks OAuth
		assert!(!config.obfuscate_sensitive_words);
		assert!(config.add_stainless_headers);
	}

	#[test]
	fn test_none_config() {
		let config = OAuthConfig::none();
		assert!(!config.inject_system_prompt);
		assert!(!config.prefix_tool_names);
		assert!(!config.inject_user_id);
	}

	#[test]
	fn test_builder_pattern() {
		let config = OAuthConfig::new()
			.with_inject_user_id(true)
			.with_obfuscation(true)
			.with_system_prompt_text("Custom prompt");

		assert!(config.inject_user_id);
		assert!(config.obfuscate_sensitive_words);
		assert_eq!(config.get_system_prompt_text(), "Custom prompt");
	}
}
