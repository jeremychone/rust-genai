//! OAuth utility functions and constants for Claude Code CLI authentication.
//!
//! This module provides helper functions for OAuth token handling including
//! tool name prefixing, user ID generation, and system prompt injection.

use crate::resolver::OAuthCredentials;

// Re-export CLAUDE_CODE_SYSTEM_TEXT for backward compatibility
#[allow(unused_imports)]
pub use crate::resolver::CLAUDE_CODE_SYSTEM_TEXT;

/// Prefix added to tool names for OAuth requests.
pub const OAUTH_TOOL_PREFIX: &str = "proxy_";

/// Anthropic-beta header value for OAuth requests.
/// Minimal value that works with OAuth tokens.
pub const OAUTH_ANTHROPIC_BETA: &str = "oauth-2025-04-20";

/// User-Agent header value mimicking Claude Code CLI.
/// Reserved for future use with full cloaking mode.
#[allow(dead_code)]
pub const CLAUDE_CLI_USER_AGENT: &str = "claude-cli/1.0.83 (external, cli)";

/// Returns headers that mimic the Claude Code CLI client.
///
/// These headers make requests appear as if they come from the official Claude CLI.
/// Reserved for future use with full cloaking mode.
#[allow(dead_code)]
pub fn get_claude_cli_headers() -> Vec<(&'static str, &'static str)> {
	vec![
		("X-App", "cli"),
		("X-Stainless-Helper-Method", "stream"),
		("X-Stainless-Retry-Count", "0"),
		("X-Stainless-Runtime-Version", "v24.3.0"),
		("X-Stainless-Package-Version", "0.55.1"),
		("X-Stainless-Runtime", "node"),
		("X-Stainless-Lang", "js"),
		("X-Stainless-Arch", "x64"),
		("X-Stainless-Os", "Windows"),
		("X-Stainless-Timeout", "600"),
		("User-Agent", CLAUDE_CLI_USER_AGENT),
	]
}

/// Check if a token string is an OAuth token.
///
/// OAuth tokens from Claude Code CLI start with `sk-ant-oat-`.
/// Delegates to [`OAuthCredentials::is_oauth_token`].
#[inline]
pub fn is_oauth_token(token: &str) -> bool {
	OAuthCredentials::is_oauth_token(token)
}

/// Add the `proxy_` prefix to a tool name.
#[inline]
pub fn prefix_tool_name(name: &str) -> String {
	format!("{}{}", OAUTH_TOOL_PREFIX, name)
}

/// Strip the `proxy_` prefix from a tool name if present.
#[inline]
pub fn strip_tool_prefix(name: &str) -> String {
	name.strip_prefix(OAUTH_TOOL_PREFIX)
		.map(|s| s.to_string())
		.unwrap_or_else(|| name.to_string())
}

/// Generate a fake user ID in the format expected by Claude Code OAuth.
///
/// Format: `user_{64-hex-chars}_account__session_{uuid-v4}`
///
/// Uses cryptographic random for the hex part and UUID v4 for the session.
///
/// Note: Currently unused as basic OAuth works without user_id injection.
/// Reserved for future use if Anthropic adds stricter requirements.
#[allow(dead_code)]
pub fn generate_fake_user_id() -> String {
	// Generate 32 random bytes -> 64 hex chars
	let mut hex_bytes = [0u8; 32];
	getrandom::getrandom(&mut hex_bytes).expect("Failed to generate random bytes");

	let hex_part: String = hex_bytes.iter().map(|b| format!("{:02x}", b)).collect();

	// Generate UUID v4 for session
	let uuid_part = uuid::Uuid::new_v4().to_string();

	format!("user_{}_account__session_{}", hex_part, uuid_part)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_is_oauth_token() {
		assert!(is_oauth_token("sk-ant-oat-abc123"));
		assert!(is_oauth_token("sk-ant-oat-SFMyNTY"));
		assert!(is_oauth_token("sk-ant-oat01-abc123")); // New format
		// Should NOT match tokens that don't start with the prefix
		assert!(!is_oauth_token("prefix-sk-ant-oat-abc123"));
		assert!(!is_oauth_token("sk-ant-api01-abc123"));
		assert!(!is_oauth_token("regular-api-key"));
	}

	#[test]
	fn test_prefix_tool_name() {
		assert_eq!(prefix_tool_name("my_tool"), "proxy_my_tool");
		assert_eq!(prefix_tool_name(""), "proxy_");
	}

	#[test]
	fn test_strip_tool_prefix() {
		assert_eq!(strip_tool_prefix("proxy_my_tool"), "my_tool");
		assert_eq!(strip_tool_prefix("my_tool"), "my_tool");
		assert_eq!(strip_tool_prefix("proxy_"), "");
	}

	#[test]
	fn test_generate_fake_user_id_format() {
		let user_id = generate_fake_user_id();

		// Check format: user_{64-hex}_account__session_{uuid}
		assert!(user_id.starts_with("user_"));
		assert!(user_id.contains("_account__session_"));

		let parts: Vec<&str> = user_id.split("_account__session_").collect();
		assert_eq!(parts.len(), 2);

		// Check hex part length (user_ prefix + 64 chars)
		let hex_part = parts[0].strip_prefix("user_").unwrap();
		assert_eq!(hex_part.len(), 64);

		// Check that hex part contains only hex characters
		assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
	}

	#[test]
	fn test_generate_fake_user_id_uniqueness() {
		// With cryptographic random, consecutive calls should always be unique
		let id1 = generate_fake_user_id();
		let id2 = generate_fake_user_id();
		let id3 = generate_fake_user_id();
		assert_ne!(id1, id2);
		assert_ne!(id2, id3);
		assert_ne!(id1, id3);
	}
}
