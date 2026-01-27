//! OAuth request and response transformers for Claude Code CLI authentication.
//!
//! This module handles the automatic transformations required for OAuth tokens:
//! - Request: inject user_id, prepend system prompt, prefix tool names
//! - Response: strip tool prefixes from tool call names

use super::oauth_utils::{generate_fake_user_id, prefix_tool_name, strip_tool_prefix, CLAUDE_CODE_SYSTEM_PROMPT, OAUTH_TOOL_PREFIX};
use serde_json::{Value, json};
use value_ext::JsonValueExt;

/// Transformer for OAuth requests.
///
/// Applies the following transformations when `is_oauth` is true:
/// 1. Injects a fake `metadata.user_id` if not present
/// 2. Prepends the Claude Code system prompt
/// 3. Prefixes all tool names with `proxy_`
pub struct OAuthRequestTransformer;

impl OAuthRequestTransformer {
	/// Transform a request payload for OAuth authentication.
	///
	/// If `is_oauth` is false, returns the payload unchanged.
	pub fn transform(mut payload: Value, is_oauth: bool) -> Value {
		if !is_oauth {
			return payload;
		}

		Self::inject_fake_user_id(&mut payload);
		Self::inject_system_prompt(&mut payload);
		Self::prefix_tool_names(&mut payload);

		payload
	}

	/// Inject a fake user_id into metadata if not present.
	fn inject_fake_user_id(payload: &mut Value) {
		// Check if metadata.user_id already exists
		if payload.x_get::<String>("/metadata/user_id").is_ok() {
			return;
		}

		let user_id = generate_fake_user_id();

		// Get or create metadata object
		if payload.get("metadata").is_none() {
			let _ = payload.x_insert("metadata", json!({}));
		}

		let _ = payload.x_insert("/metadata/user_id", user_id);
	}

	/// Prepend the Claude Code system prompt to the existing system.
	fn inject_system_prompt(payload: &mut Value) {
		match payload.get("system") {
			Some(Value::String(existing)) => {
				let new_system = format!("{}{}", CLAUDE_CODE_SYSTEM_PROMPT, existing);
				let _ = payload.x_insert("system", new_system);
			}
			Some(Value::Array(parts)) => {
				// System is an array of parts, prepend new text part
				let mut new_parts = vec![json!({
					"type": "text",
					"text": CLAUDE_CODE_SYSTEM_PROMPT
				})];
				new_parts.extend(parts.iter().cloned());
				let _ = payload.x_insert("system", new_parts);
			}
			None => {
				// No system prompt, just add ours
				let _ = payload.x_insert("system", CLAUDE_CODE_SYSTEM_PROMPT);
			}
			_ => {
				// Unknown format, skip
			}
		}
	}

	/// Prefix all tool names with `proxy_`.
	fn prefix_tool_names(payload: &mut Value) {
		if let Some(tools) = payload.get_mut("tools") {
			if let Some(tools_array) = tools.as_array_mut() {
				for tool in tools_array.iter_mut() {
					if let Some(name) = tool.get_mut("name") {
						if let Some(name_str) = name.as_str() {
							*name = json!(prefix_tool_name(name_str));
						}
					}
				}
			}
		}
	}
}

/// Transformer for OAuth responses.
///
/// Applies the following transformations when `is_oauth` is true:
/// - Strips `proxy_` prefix from tool call names
pub struct OAuthResponseTransformer;

impl OAuthResponseTransformer {
	/// Transform a response body for OAuth authentication.
	///
	/// If `is_oauth` is false, returns the body unchanged.
	pub fn transform(mut body: Value, is_oauth: bool) -> Value {
		if !is_oauth {
			return body;
		}

		Self::strip_tool_prefixes(&mut body);
		body
	}

	/// Strip `proxy_` prefix from tool_use content items.
	fn strip_tool_prefixes(body: &mut Value) {
		if let Some(content) = body.get_mut("content") {
			if let Some(content_array) = content.as_array_mut() {
				for item in content_array.iter_mut() {
					// Check if this is a tool_use item
					if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
						if let Some(name) = item.get_mut("name") {
							if let Some(name_str) = name.as_str() {
								if name_str.starts_with(OAUTH_TOOL_PREFIX) {
									*name = json!(strip_tool_prefix(name_str));
								}
							}
						}
					}
				}
			}
		}
	}

	/// Check if a response appears to be from an OAuth request by checking tool names.
	///
	/// Returns true if any tool_use content item has a name starting with `proxy_`.
	pub fn detect_oauth_response(body: &Value) -> bool {
		if let Some(content) = body.get("content") {
			if let Some(content_array) = content.as_array() {
				for item in content_array.iter() {
					if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
						if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
							if name.starts_with(OAUTH_TOOL_PREFIX) {
								return true;
							}
						}
					}
				}
			}
		}
		false
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_request_transform_no_oauth() {
		let payload = json!({
			"model": "claude-3",
			"messages": []
		});

		let result = OAuthRequestTransformer::transform(payload.clone(), false);
		assert_eq!(result, payload);
	}

	#[test]
	fn test_request_transform_injects_user_id() {
		let payload = json!({
			"model": "claude-3",
			"messages": []
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		assert!(result.get("metadata").is_some());
		let user_id = result.x_get::<String>("/metadata/user_id").unwrap();
		assert!(user_id.starts_with("user_"));
		assert!(user_id.contains("_account__session_"));
	}

	#[test]
	fn test_request_transform_preserves_existing_user_id() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"metadata": {
				"user_id": "existing_user_id"
			}
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		let user_id = result.x_get::<String>("/metadata/user_id").unwrap();
		assert_eq!(user_id, "existing_user_id");
	}

	#[test]
	fn test_request_transform_injects_system_prompt() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"system": "Original system prompt"
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		let system = result.get("system").unwrap().as_str().unwrap();
		assert!(system.starts_with("You are Claude Code"));
		assert!(system.contains("Original system prompt"));
	}

	#[test]
	fn test_request_transform_adds_system_when_missing() {
		let payload = json!({
			"model": "claude-3",
			"messages": []
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		assert!(result.get("system").is_some());
	}

	#[test]
	fn test_request_transform_prefixes_tools() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"tools": [
				{"name": "read_file", "input_schema": {}},
				{"name": "write_file", "input_schema": {}}
			]
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		let tools = result.get("tools").unwrap().as_array().unwrap();
		assert_eq!(tools[0].get("name").unwrap().as_str().unwrap(), "proxy_read_file");
		assert_eq!(tools[1].get("name").unwrap().as_str().unwrap(), "proxy_write_file");
	}

	#[test]
	fn test_response_transform_no_oauth() {
		let body = json!({
			"content": [
				{"type": "tool_use", "name": "proxy_my_tool", "id": "123", "input": {}}
			]
		});

		let result = OAuthResponseTransformer::transform(body.clone(), false);
		assert_eq!(result, body);
	}

	#[test]
	fn test_response_transform_strips_tool_prefix() {
		let body = json!({
			"content": [
				{"type": "text", "text": "Hello"},
				{"type": "tool_use", "name": "proxy_my_tool", "id": "123", "input": {}}
			]
		});

		let result = OAuthResponseTransformer::transform(body, true);

		let content = result.get("content").unwrap().as_array().unwrap();
		assert_eq!(content[1].get("name").unwrap().as_str().unwrap(), "my_tool");
	}

	#[test]
	fn test_detect_oauth_response() {
		let oauth_body = json!({
			"content": [
				{"type": "tool_use", "name": "proxy_my_tool", "id": "123", "input": {}}
			]
		});

		let normal_body = json!({
			"content": [
				{"type": "tool_use", "name": "my_tool", "id": "123", "input": {}}
			]
		});

		assert!(OAuthResponseTransformer::detect_oauth_response(&oauth_body));
		assert!(!OAuthResponseTransformer::detect_oauth_response(&normal_body));
	}
}
