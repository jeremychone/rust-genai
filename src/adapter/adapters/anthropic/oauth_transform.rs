//! OAuth request and response transformers for Claude Code CLI authentication.
//!
//! This module handles the automatic transformations required for OAuth tokens:
//! - Request: inject system prompt, prefix tool names, optionally inject user_id and obfuscate
//! - Response: strip tool prefixes from tool call names

use super::oauth_config::OAuthConfig;
use super::oauth_obfuscate::{obfuscate_payload, DEFAULT_SENSITIVE_WORDS};
use super::oauth_utils::{generate_fake_user_id, prefix_tool_name, strip_tool_prefix, OAUTH_TOOL_PREFIX};
use serde_json::{Value, json};
use value_ext::JsonValueExt;

/// Transformer for OAuth requests.
///
/// Applies transformations based on OAuthConfig when `is_oauth` is true:
/// - System prompt injection (required)
/// - Tool name prefixing (required)
/// - User ID injection (optional)
/// - Sensitive word obfuscation (optional)
pub struct OAuthRequestTransformer;

impl OAuthRequestTransformer {
	/// Transform a request payload for OAuth authentication using default config.
	///
	/// If `is_oauth` is false, returns the payload unchanged.
	pub fn transform(payload: Value, is_oauth: bool) -> Value {
		Self::transform_with_config(payload, is_oauth, &OAuthConfig::default())
	}

	/// Transform a request payload for OAuth authentication with custom config.
	///
	/// If `is_oauth` is false, returns the payload unchanged.
	pub fn transform_with_config(mut payload: Value, is_oauth: bool, config: &OAuthConfig) -> Value {
		if !is_oauth {
			return payload;
		}

		// System prompt injection
		if config.inject_system_prompt {
			Self::inject_system_prompt(&mut payload, config);
		}

		// Tool name prefixing
		if config.prefix_tool_names {
			Self::prefix_tool_names(&mut payload);
		}

		// User ID injection (optional)
		if config.inject_user_id {
			Self::inject_fake_user_id(&mut payload);
		}

		// Sensitive word obfuscation (optional)
		if config.obfuscate_sensitive_words {
			obfuscate_payload(&mut payload, DEFAULT_SENSITIVE_WORDS);
		}

		payload
	}

	/// Inject a fake user_id into metadata if not present.
	fn inject_fake_user_id(payload: &mut Value) {
		if payload.x_get::<String>("/metadata/user_id").is_ok() {
			return;
		}

		let user_id = generate_fake_user_id();

		if payload.get("metadata").is_none() {
			let _ = payload.x_insert("metadata", json!({}));
		}

		let _ = payload.x_insert("/metadata/user_id", user_id);
	}

	/// Prepend the Claude Code system prompt to the existing system.
	/// Uses JSON array format with cache_control on the LAST element as required by Claude Code OAuth.
	fn inject_system_prompt(payload: &mut Value, config: &OAuthConfig) {
		let system_text = config.get_system_prompt_text();

		// Claude Code prompt (without cache_control - it goes on the last element)
		let claude_code_part = json!({
			"type": "text",
			"text": system_text
		});

		match payload.get("system") {
			Some(Value::String(existing)) => {
				// Convert string to array format
				let mut existing_part = json!({
					"type": "text",
					"text": existing
				});
				// Add cache_control to last element if enabled
				if config.use_cache_control {
					if let Some(obj) = existing_part.as_object_mut() {
						obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
					}
				}
				let _ = payload.x_insert("system", json!([claude_code_part, existing_part]));
			}
			Some(Value::Array(parts)) => {
				// System is already an array, prepend Claude Code prompt
				let mut new_parts = vec![claude_code_part];
				let mut parts_clone: Vec<Value> = parts.iter().cloned().collect();
				// Add cache_control to the last element if enabled
				if config.use_cache_control {
					if let Some(last) = parts_clone.last_mut() {
						if last.get("cache_control").is_none() {
							if let Some(obj) = last.as_object_mut() {
								obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
							}
						}
					}
				}
				new_parts.extend(parts_clone);
				let _ = payload.x_insert("system", new_parts);
			}
			None => {
				// No system prompt, add ours
				let mut claude_code_only = json!({
					"type": "text",
					"text": system_text
				});
				if config.use_cache_control {
					if let Some(obj) = claude_code_only.as_object_mut() {
						obj.insert("cache_control".to_string(), json!({"type": "ephemeral"}));
					}
				}
				let _ = payload.x_insert("system", json!([claude_code_only]));
			}
			_ => {
				// Unknown format, skip
			}
		}
	}

	/// Prefix all tool names with `proxy_`.
	///
	/// Handles:
	/// - `tools[].name` - tool definitions (skips built-in tools with "type" field)
	/// - `tool_choice.name` - when forcing a specific tool
	/// - `messages[*].content[*].name` - tool_use in previous assistant messages
	fn prefix_tool_names(payload: &mut Value) {
		// 1. Prefix tools[].name (skip built-in tools that have a "type" field)
		if let Some(tools) = payload.get_mut("tools") {
			if let Some(tools_array) = tools.as_array_mut() {
				for tool in tools_array.iter_mut() {
					// Skip built-in tools (web_search, code_execution, etc.)
					// which have a "type" field
					if let Some(tool_type) = tool.get("type") {
						if tool_type.as_str().map(|s| !s.is_empty()).unwrap_or(false) {
							continue;
						}
					}

					if let Some(name) = tool.get_mut("name") {
						if let Some(name_str) = name.as_str() {
							// Skip if already prefixed
							if !name_str.starts_with(OAUTH_TOOL_PREFIX) {
								*name = json!(prefix_tool_name(name_str));
							}
						}
					}
				}
			}
		}

		// 2. Prefix tool_choice.name when tool_choice.type == "tool"
		if let Some(tool_choice) = payload.get_mut("tool_choice") {
			if tool_choice.get("type").and_then(|t| t.as_str()) == Some("tool") {
				if let Some(name) = tool_choice.get_mut("name") {
					if let Some(name_str) = name.as_str() {
						if !name_str.starts_with(OAUTH_TOOL_PREFIX) {
							*name = json!(prefix_tool_name(name_str));
						}
					}
				}
			}
		}

		// 3. Prefix messages[*].content[*].name for tool_use items
		if let Some(messages) = payload.get_mut("messages") {
			if let Some(messages_array) = messages.as_array_mut() {
				for message in messages_array.iter_mut() {
					if let Some(content) = message.get_mut("content") {
						if let Some(content_array) = content.as_array_mut() {
							for part in content_array.iter_mut() {
								// Only process tool_use items
								if part.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
									continue;
								}

								if let Some(name) = part.get_mut("name") {
									if let Some(name_str) = name.as_str() {
										if !name_str.starts_with(OAUTH_TOOL_PREFIX) {
											*name = json!(prefix_tool_name(name_str));
										}
									}
								}
							}
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
	fn test_request_transform_injects_system_prompt() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"system": "Original system prompt"
		});

		// Test with cache_control enabled
		let config = OAuthConfig::default().with_cache_control(true);
		let result = OAuthRequestTransformer::transform_with_config(payload, true, &config);

		// System should now be a JSON array
		let system = result.get("system").unwrap().as_array().unwrap();
		assert_eq!(system.len(), 2);

		// First element is Claude Code prompt (no cache_control)
		let first = &system[0];
		assert_eq!(first.get("type").unwrap().as_str().unwrap(), "text");
		assert!(first.get("text").unwrap().as_str().unwrap().contains("Claude Code"));
		assert!(first.get("cache_control").is_none());

		// Second element is the original prompt WITH cache_control (on last element)
		let second = &system[1];
		assert_eq!(second.get("type").unwrap().as_str().unwrap(), "text");
		assert_eq!(second.get("text").unwrap().as_str().unwrap(), "Original system prompt");
		assert!(second.get("cache_control").is_some());
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

		// Test with prefix_tool_names enabled
		let config = OAuthConfig::default().with_prefix_tool_names(true);
		let result = OAuthRequestTransformer::transform_with_config(payload, true, &config);

		let tools = result.get("tools").unwrap().as_array().unwrap();
		assert_eq!(tools[0].get("name").unwrap().as_str().unwrap(), "proxy_read_file");
		assert_eq!(tools[1].get("name").unwrap().as_str().unwrap(), "proxy_write_file");
	}

	#[test]
	fn test_request_transform_skips_builtin_tools() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"tools": [
				{"name": "read_file", "input_schema": {}},
				{"type": "web_search", "name": "web_search"},
				{"type": "code_execution", "name": "code_execution"}
			]
		});

		// Test with prefix_tool_names enabled
		let config = OAuthConfig::default().with_prefix_tool_names(true);
		let result = OAuthRequestTransformer::transform_with_config(payload, true, &config);

		let tools = result.get("tools").unwrap().as_array().unwrap();
		// Regular tool gets prefixed
		assert_eq!(tools[0].get("name").unwrap().as_str().unwrap(), "proxy_read_file");
		// Built-in tools (with "type" field) are NOT prefixed
		assert_eq!(tools[1].get("name").unwrap().as_str().unwrap(), "web_search");
		assert_eq!(tools[2].get("name").unwrap().as_str().unwrap(), "code_execution");
	}

	#[test]
	fn test_request_transform_prefixes_tool_choice() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"tools": [{"name": "my_tool", "input_schema": {}}],
			"tool_choice": {"type": "tool", "name": "my_tool"}
		});

		// Test with prefix_tool_names enabled
		let config = OAuthConfig::default().with_prefix_tool_names(true);
		let result = OAuthRequestTransformer::transform_with_config(payload, true, &config);

		let tool_choice = result.get("tool_choice").unwrap();
		assert_eq!(tool_choice.get("name").unwrap().as_str().unwrap(), "proxy_my_tool");
	}

	#[test]
	fn test_request_transform_prefixes_message_tool_use() {
		let payload = json!({
			"model": "claude-3",
			"messages": [
				{"role": "user", "content": "Hello"},
				{
					"role": "assistant",
					"content": [
						{"type": "text", "text": "I'll read the file"},
						{"type": "tool_use", "id": "call_123", "name": "read_file", "input": {"path": "test.txt"}}
					]
				},
				{
					"role": "user",
					"content": [
						{"type": "tool_result", "tool_use_id": "call_123", "content": "file contents"}
					]
				}
			]
		});

		// Test with prefix_tool_names enabled
		let config = OAuthConfig::default().with_prefix_tool_names(true);
		let result = OAuthRequestTransformer::transform_with_config(payload, true, &config);

		let messages = result.get("messages").unwrap().as_array().unwrap();
		// Check assistant message tool_use is prefixed
		let assistant_content = messages[1].get("content").unwrap().as_array().unwrap();
		assert_eq!(assistant_content[1].get("name").unwrap().as_str().unwrap(), "proxy_read_file");
	}

	#[test]
	fn test_request_transform_skips_already_prefixed() {
		let payload = json!({
			"model": "claude-3",
			"messages": [],
			"tools": [
				{"name": "proxy_already_prefixed", "input_schema": {}}
			]
		});

		let result = OAuthRequestTransformer::transform(payload, true);

		let tools = result.get("tools").unwrap().as_array().unwrap();
		// Should NOT double-prefix
		assert_eq!(tools[0].get("name").unwrap().as_str().unwrap(), "proxy_already_prefixed");
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
