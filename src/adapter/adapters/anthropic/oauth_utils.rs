//! OAuth utility functions and constants for Claude Code CLI authentication.
//!
//! This module provides helper functions for OAuth token handling including
//! tool name prefixing, user ID generation, and system prompt injection.

/// Prefix added to tool names for OAuth requests.
pub const OAUTH_TOOL_PREFIX: &str = "proxy_";

/// System prompt prepended to OAuth requests for Claude Code CLI.
pub const CLAUDE_CODE_SYSTEM_PROMPT: &str = r#"You are Claude Code, a coding assistant created by Anthropic.

<claude-code-system-prompt>
You help users with coding tasks by understanding their requirements, analyzing code,
suggesting implementations, and assisting with debugging. You can access tools to
read files, write code, and execute commands when appropriate.
</claude-code-system-prompt>

"#;

/// Anthropic-beta header value for OAuth requests.
pub const OAUTH_ANTHROPIC_BETA: &str = "claude-code-20250219,oauth-2025-04-20,interleaved-thinking-2025-05-14,prompt-caching-2024-07-31,pdfs-2024-09-25,token-efficient-tools-2025-02-19";

/// Check if a token string is an OAuth token.
///
/// OAuth tokens from Claude Code CLI contain `sk-ant-oat`.
#[inline]
pub fn is_oauth_token(token: &str) -> bool {
	token.contains("sk-ant-oat")
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
pub fn generate_fake_user_id() -> String {
	use std::time::{SystemTime, UNIX_EPOCH};

	// Generate pseudo-random hex string using current time and a simple hash
	let now = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_nanos())
		.unwrap_or(0);

	// Create a 64-char hex string from time-based hash
	let hex_part: String = (0..64)
		.map(|i| {
			let byte = ((now.wrapping_add(i as u128).wrapping_mul(0x5DEECE66D)) >> (i % 8)) as u8;
			format!("{:x}", byte & 0x0F)
		})
		.collect();

	// Generate a simple UUID-like string
	let uuid = format!(
		"{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
		(now >> 96) as u32,
		(now >> 80) as u16,
		(now >> 64) as u16 & 0x0FFF,
		((now >> 48) as u16 & 0x3FFF) | 0x8000,
		now as u64 & 0xFFFFFFFFFFFF
	);

	format!("user_{}_account__session_{}", hex_part, uuid)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_is_oauth_token() {
		assert!(is_oauth_token("sk-ant-oat-abc123"));
		assert!(is_oauth_token("prefix-sk-ant-oat-abc123"));
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
		let id1 = generate_fake_user_id();
		std::thread::sleep(std::time::Duration::from_millis(1));
		let id2 = generate_fake_user_id();
		assert_ne!(id1, id2);
	}
}
