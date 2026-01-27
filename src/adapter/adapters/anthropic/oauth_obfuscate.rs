//! Sensitive word obfuscation for OAuth cloaking.
//!
//! This module provides functions to obfuscate sensitive words in requests
//! by inserting zero-width space characters. This makes the text appear
//! unchanged to humans but breaks simple string matching.
//!
//! **IMPORTANT:** The default word list does NOT include "claude" or "anthropic"
//! because the Anthropic API validates the exact text of the system prompt.
//! Obfuscating these words would cause OAuth authentication to fail.

#![allow(dead_code)]

use serde_json::Value;

/// Zero-width space character used for obfuscation.
const ZERO_WIDTH_SPACE: char = '\u{200B}';

/// Default list of sensitive words to obfuscate.
///
/// Empty by default - user should specify their own list if needed.
///
/// **IMPORTANT:** Do NOT include "claude" or "anthropic"!
/// These words are in the Claude Code system prompt and obfuscating them
/// breaks OAuth authentication (API validates exact system prompt text).
pub const DEFAULT_SENSITIVE_WORDS: &[&str] = &[
	// Empty by default, similar to CLIProxyAPI
	// User can specify custom words via OAuthConfig if needed
];

/// Obfuscates a single word by inserting a zero-width space after the first character.
///
/// Example: "proxy" → "p​roxy" (with invisible zero-width space)
pub fn obfuscate_word(word: &str) -> String {
	if word.contains(ZERO_WIDTH_SPACE) || word.chars().count() < 2 {
		return word.to_string();
	}

	let mut chars = word.chars();
	if let Some(first) = chars.next() {
		format!("{}{}{}", first, ZERO_WIDTH_SPACE, chars.collect::<String>())
	} else {
		word.to_string()
	}
}

/// Obfuscates all occurrences of sensitive words in a text.
///
/// The matching is case-insensitive, but the original case is preserved.
pub fn obfuscate_text(text: &str, words: &[&str]) -> String {
	if words.is_empty() {
		return text.to_string();
	}

	let mut result = text.to_string();

	for word in words {
		if word.is_empty() || word.contains(ZERO_WIDTH_SPACE) {
			continue;
		}

		// Case-insensitive search and replace
		let lower_word = word.to_lowercase();
		let mut search_start = 0;

		while let Some(pos) = result[search_start..].to_lowercase().find(&lower_word) {
			let actual_pos = search_start + pos;
			let actual_word = &result[actual_pos..actual_pos + word.len()];

			// Check if already obfuscated
			if !actual_word.contains(ZERO_WIDTH_SPACE) {
				let obfuscated = obfuscate_word(actual_word);
				result = format!(
					"{}{}{}",
					&result[..actual_pos],
					obfuscated,
					&result[actual_pos + word.len()..]
				);
				// Move past the obfuscated word (which is now longer due to ZWSP)
				search_start = actual_pos + obfuscated.len();
			} else {
				search_start = actual_pos + word.len();
			}
		}
	}

	result
}

/// Obfuscates sensitive words in a JSON payload.
///
/// Processes:
/// - `system` (string or array of text parts)
/// - `messages[*].content` (string or array of text parts)
pub fn obfuscate_payload(payload: &mut Value, words: &[&str]) {
	if words.is_empty() {
		return;
	}

	// Obfuscate system prompt
	obfuscate_system(payload, words);

	// Obfuscate messages
	obfuscate_messages(payload, words);
}

/// Obfuscates sensitive words in the system prompt.
fn obfuscate_system(payload: &mut Value, words: &[&str]) {
	if let Some(system) = payload.get_mut("system") {
		match system {
			Value::String(text) => {
				*text = obfuscate_text(text, words);
			}
			Value::Array(parts) => {
				for part in parts.iter_mut() {
					if part.get("type").and_then(|t| t.as_str()) == Some("text") {
						if let Some(text) = part.get_mut("text") {
							if let Some(text_str) = text.as_str() {
								*text = Value::String(obfuscate_text(text_str, words));
							}
						}
					}
				}
			}
			_ => {}
		}
	}
}

/// Obfuscates sensitive words in message content.
fn obfuscate_messages(payload: &mut Value, words: &[&str]) {
	if let Some(messages) = payload.get_mut("messages") {
		if let Some(messages_array) = messages.as_array_mut() {
			for message in messages_array.iter_mut() {
				if let Some(content) = message.get_mut("content") {
					match content {
						Value::String(text) => {
							*text = obfuscate_text(text, words);
						}
						Value::Array(parts) => {
							for part in parts.iter_mut() {
								if part.get("type").and_then(|t| t.as_str()) == Some("text") {
									if let Some(text) = part.get_mut("text") {
										if let Some(text_str) = text.as_str() {
											*text = Value::String(obfuscate_text(text_str, words));
										}
									}
								}
							}
						}
						_ => {}
					}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_obfuscate_word() {
		let result = obfuscate_word("proxy");
		assert_eq!(result.chars().count(), 6); // 5 chars + 1 ZWSP
		assert!(result.contains(ZERO_WIDTH_SPACE));
		assert!(result.starts_with('p'));
	}

	#[test]
	fn test_obfuscate_word_already_obfuscated() {
		let already = format!("p{}roxy", ZERO_WIDTH_SPACE);
		let result = obfuscate_word(&already);
		assert_eq!(result, already); // Should not double-obfuscate
	}

	#[test]
	fn test_obfuscate_word_short() {
		assert_eq!(obfuscate_word("a"), "a"); // Too short to obfuscate
		assert_eq!(obfuscate_word(""), "");
	}

	#[test]
	fn test_obfuscate_text() {
		let text = "Using a proxy to access the API";
		let result = obfuscate_text(text, &["proxy", "api"]);

		// Original words should be obfuscated
		assert!(result.contains(ZERO_WIDTH_SPACE));
		// Text should still be readable (minus the invisible chars)
		let visible: String = result.chars().filter(|c| *c != ZERO_WIDTH_SPACE).collect();
		assert_eq!(visible, text);
	}

	#[test]
	fn test_obfuscate_text_case_insensitive() {
		let text = "PROXY and Proxy and proxy";
		let result = obfuscate_text(text, &["proxy"]);

		// Count ZWSP - should be 3 (one for each occurrence)
		let zwsp_count = result.chars().filter(|c| *c == ZERO_WIDTH_SPACE).count();
		assert_eq!(zwsp_count, 3);
	}

	#[test]
	fn test_obfuscate_payload_system_string() {
		let mut payload = json!({
			"model": "claude-3",
			"system": "Using proxy to access anthropic",
			"messages": []
		});

		obfuscate_payload(&mut payload, &["proxy", "anthropic"]);

		let system = payload.get("system").unwrap().as_str().unwrap();
		assert!(system.contains(ZERO_WIDTH_SPACE));
	}

	#[test]
	fn test_obfuscate_payload_system_array() {
		let mut payload = json!({
			"model": "claude-3",
			"system": [
				{"type": "text", "text": "Using proxy"},
				{"type": "text", "text": "to access anthropic"}
			],
			"messages": []
		});

		obfuscate_payload(&mut payload, &["proxy", "anthropic"]);

		let system = payload.get("system").unwrap().as_array().unwrap();
		let text1 = system[0].get("text").unwrap().as_str().unwrap();
		let text2 = system[1].get("text").unwrap().as_str().unwrap();
		assert!(text1.contains(ZERO_WIDTH_SPACE));
		assert!(text2.contains(ZERO_WIDTH_SPACE));
	}

	#[test]
	fn test_obfuscate_payload_messages() {
		let mut payload = json!({
			"model": "claude-3",
			"messages": [
				{"role": "user", "content": "Tell me about proxy servers"},
				{"role": "assistant", "content": [
					{"type": "text", "text": "Proxy servers are..."}
				]}
			]
		});

		obfuscate_payload(&mut payload, &["proxy"]);

		let messages = payload.get("messages").unwrap().as_array().unwrap();

		// Check user message
		let user_content = messages[0].get("content").unwrap().as_str().unwrap();
		assert!(user_content.contains(ZERO_WIDTH_SPACE));

		// Check assistant message
		let assistant_content = messages[1].get("content").unwrap().as_array().unwrap();
		let text = assistant_content[0].get("text").unwrap().as_str().unwrap();
		assert!(text.contains(ZERO_WIDTH_SPACE));
	}

	#[test]
	fn test_obfuscate_payload_empty_words() {
		let mut payload = json!({
			"model": "claude-3",
			"system": "Using proxy",
			"messages": []
		});

		let original = payload.clone();
		obfuscate_payload(&mut payload, &[]);

		assert_eq!(payload, original); // No changes
	}
}
