//! Helpers to derive OTel attribute values from genai types.
//!
//! This covers `server.address` / `server.port` parsing, the opt-in
//! content-capture gate, and the JSON serializers for the (opt-in, sensitive)
//! message-content attributes (`gen_ai.input.messages`, `gen_ai.output.messages`,
//! `gen_ai.system_instructions`, `gen_ai.tool.definitions`).

use crate::chat::{ChatMessage, ChatRole, ContentPart, MessageContent, StopReason, Tool};
use serde_json::{Value, json};
use std::sync::OnceLock;

// region:    --- Content capture gate

/// Environment variable that opts into capturing prompt/response content on
/// GenAI spans. Matches the name used by the OpenTelemetry GenAI instrumentations.
///
/// Accepted truthy values (case-insensitive): `true`, `1`, `yes`, `on`.
pub const CAPTURE_CONTENT_ENV: &str = "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT";

/// Returns `true` when message content should be recorded on spans.
///
/// Content is **off by default** because it is likely to contain user/PII data
/// (see the spec warnings). Enable it by setting [`CAPTURE_CONTENT_ENV`].
/// The value is read once and cached for the process lifetime.
pub fn capture_content() -> bool {
	static CAPTURE: OnceLock<bool> = OnceLock::new();
	*CAPTURE.get_or_init(|| {
		std::env::var(CAPTURE_CONTENT_ENV)
			.map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
			.unwrap_or(false)
	})
}

// endregion: --- Content capture gate

// region:    --- server.address / server.port

/// Parses a base URL into `(server.address, server.port)`.
///
/// Best-effort, dependency-free parse (genai does not depend on the `url`
/// crate). When the port is not explicit it is inferred from the scheme
/// (`https` → 443, `http` → 80).
pub fn server_address_and_port(base_url: &str) -> (Option<String>, Option<u16>) {
	// -- Split off the scheme.
	let (scheme, rest) = match base_url.split_once("://") {
		Some((scheme, rest)) => (Some(scheme), rest),
		None => (None, base_url),
	};

	// -- Authority is everything up to the first '/', '?' or '#'.
	let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest).trim_end_matches('.');

	if authority.is_empty() {
		return (None, None);
	}

	// -- Split host and optional port. Guard against IPv6 (`[::1]:443`).
	let (host, explicit_port) = if let Some(stripped) = authority.strip_prefix('[') {
		// IPv6 literal: `[host]` or `[host]:port`.
		match stripped.split_once(']') {
			Some((host, after)) => (host.to_string(), after.trim_start_matches(':')),
			None => (stripped.to_string(), ""),
		}
	} else if let Some((host, port)) = authority.rsplit_once(':') {
		(host.to_string(), port)
	} else {
		(authority.to_string(), "")
	};

	if host.is_empty() {
		return (None, None);
	}

	let port = explicit_port.parse::<u16>().ok().or(match scheme {
		Some("https") => Some(443),
		Some("http") => Some(80),
		_ => None,
	});

	(Some(host), port)
}

// endregion: --- server.address / server.port

// region:    --- finish reasons

/// Renders a single normalized stop reason as the `gen_ai.response.finish_reasons`
/// JSON array string (genai surfaces a single reason per response).
pub fn finish_reasons_json(stop_reason: &StopReason) -> String {
	json!([stop_reason.raw()]).to_string()
}

// endregion: --- finish reasons

// region:    --- Content serializers

/// Lower-case `role` value per the message JSON schema.
fn role_str(role: &ChatRole) -> &'static str {
	match role {
		ChatRole::System => "system",
		ChatRole::User => "user",
		ChatRole::Assistant => "assistant",
		ChatRole::Tool => "tool",
	}
}

/// Maps a single [`ContentPart`] to a message `part` object, or `None` for parts
/// that have no meaningful (non-internal) representation.
fn part_value(part: &ContentPart) -> Option<Value> {
	match part {
		ContentPart::Text(text) => Some(json!({ "type": "text", "content": text })),
		ContentPart::ToolCall(tc) => Some(json!({
			"type": "tool_call",
			"id": tc.call_id,
			"name": tc.fn_name,
			"arguments": tc.fn_arguments,
		})),
		ContentPart::ToolResponse(tr) => Some(json!({
			"type": "tool_call_response",
			"id": tr.call_id,
			"result": tr.content,
		})),
		ContentPart::ReasoningContent(text) => Some(json!({ "type": "reasoning", "content": text })),
		// Opaque/internal parts: represented by type only (no content emitted).
		ContentPart::Binary(_) => Some(json!({ "type": "blob" })),
		ContentPart::ThoughtSignature(_) | ContentPart::Custom(_) => None,
	}
}

fn parts_value(content: &MessageContent) -> Value {
	let parts: Vec<Value> = content.parts().iter().filter_map(part_value).collect();
	Value::Array(parts)
}

/// `gen_ai.input.messages` — the chat history sent to the model.
///
/// System-role messages are excluded here; they belong in
/// [`system_instructions_json`] per the spec.
pub fn input_messages_json(messages: &[ChatMessage]) -> String {
	let msgs: Vec<Value> = messages
		.iter()
		.filter(|m| m.role != ChatRole::System)
		.map(|m| {
			json!({
				"role": role_str(&m.role),
				"parts": parts_value(&m.content),
			})
		})
		.collect();
	Value::Array(msgs).to_string()
}

/// `gen_ai.output.messages` — the message(s) returned by the model.
///
/// genai surfaces a single assistant choice, rendered as one message.
pub fn output_messages_json(content: &MessageContent, finish_reason: Option<&StopReason>) -> String {
	let mut msg = json!({
		"role": "assistant",
		"parts": parts_value(content),
	});
	if let Some(reason) = finish_reason {
		msg["finish_reason"] = Value::String(reason.raw().to_string());
	}
	Value::Array(vec![msg]).to_string()
}

/// `gen_ai.system_instructions` — system content provided separately from the
/// chat history, as an array of `{ "type": "text", "content": .. }` objects.
pub fn system_instructions_json<'a>(systems: impl Iterator<Item = &'a str>) -> Option<String> {
	let instructions: Vec<Value> = systems.map(|s| json!({ "type": "text", "content": s })).collect();
	if instructions.is_empty() {
		None
	} else {
		Some(Value::Array(instructions).to_string())
	}
}

/// `gen_ai.tool.definitions` — the tools made available to the model.
pub fn tool_definitions_json(tools: &[Tool]) -> String {
	let defs: Vec<Value> = tools
		.iter()
		.map(|tool| {
			let mut def = json!({
				"type": "function",
				"name": tool.name.as_str(),
			});
			if let Some(description) = &tool.description {
				def["description"] = Value::String(description.clone());
			}
			if let Some(schema) = &tool.schema {
				def["parameters"] = schema.clone();
			}
			def
		})
		.collect();
	Value::Array(defs).to_string()
}

// endregion: --- Content serializers

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chat::{ChatMessage, Tool, ToolCall};
	use serde_json::json;

	#[test]
	fn test_otel_server_address_and_port() {
		assert_eq!(
			server_address_and_port("https://api.openai.com/v1/"),
			(Some("api.openai.com".to_string()), Some(443))
		);
		assert_eq!(
			server_address_and_port("http://localhost:11434/v1/"),
			(Some("localhost".to_string()), Some(11434))
		);
		assert_eq!(
			server_address_and_port("https://generativelanguage.googleapis.com"),
			(Some("generativelanguage.googleapis.com".to_string()), Some(443))
		);
		// No scheme → no inferred port.
		assert_eq!(
			server_address_and_port("example.com"),
			(Some("example.com".to_string()), None)
		);
		// IPv6 literal with explicit port.
		assert_eq!(
			server_address_and_port("https://[::1]:8443/v1"),
			(Some("::1".to_string()), Some(8443))
		);
		assert_eq!(server_address_and_port(""), (None, None));
	}

	#[test]
	fn test_otel_finish_reasons_json() {
		let reason = StopReason::Completed("stop".to_string());
		assert_eq!(finish_reasons_json(&reason), r#"["stop"]"#);
		let reason = StopReason::MaxTokens("length".to_string());
		assert_eq!(finish_reasons_json(&reason), r#"["length"]"#);
	}

	#[test]
	fn test_otel_input_messages_json_excludes_system() {
		let messages = vec![
			ChatMessage::system("be brief"),
			ChatMessage::user("Weather in Paris?"),
			ChatMessage::assistant("It is rainy."),
		];
		let json_str = input_messages_json(&messages);
		let value: Value = serde_json::from_str(&json_str).unwrap();

		// System message is excluded; user + assistant remain.
		assert_eq!(
			value,
			json!([
				{ "role": "user", "parts": [{ "type": "text", "content": "Weather in Paris?" }] },
				{ "role": "assistant", "parts": [{ "type": "text", "content": "It is rainy." }] },
			])
		);
	}

	#[test]
	fn test_otel_input_messages_json_tool_call_parts() {
		let tool_call = ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({ "location": "Paris" }),
			thought_signatures: None,
		};
		let messages = vec![ChatMessage::from(vec![tool_call])];
		let value: Value = serde_json::from_str(&input_messages_json(&messages)).unwrap();

		assert_eq!(
			value,
			json!([
				{
					"role": "assistant",
					"parts": [{
						"type": "tool_call",
						"id": "call_1",
						"name": "get_weather",
						"arguments": { "location": "Paris" },
					}],
				},
			])
		);
	}

	#[test]
	fn test_otel_output_messages_json_with_finish_reason() {
		let content = crate::chat::MessageContent::from_text("Hello!");
		let reason = StopReason::Completed("stop".to_string());
		let value: Value = serde_json::from_str(&output_messages_json(&content, Some(&reason))).unwrap();

		assert_eq!(
			value,
			json!([
				{
					"role": "assistant",
					"parts": [{ "type": "text", "content": "Hello!" }],
					"finish_reason": "stop",
				},
			])
		);
	}

	#[test]
	fn test_otel_system_instructions_json() {
		let systems = ["You are a translator.", "Translate to French."];
		let json_str = system_instructions_json(systems.iter().copied()).unwrap();
		let value: Value = serde_json::from_str(&json_str).unwrap();

		assert_eq!(
			value,
			json!([
				{ "type": "text", "content": "You are a translator." },
				{ "type": "text", "content": "Translate to French." },
			])
		);

		// Empty → None.
		assert!(system_instructions_json(std::iter::empty()).is_none());
	}

	#[test]
	fn test_otel_tool_definitions_json() {
		let tool = Tool::new("get_weather")
			.with_description("Get the weather")
			.with_schema(json!({ "type": "object", "properties": { "location": { "type": "string" } } }));
		let value: Value = serde_json::from_str(&tool_definitions_json(&[tool])).unwrap();

		assert_eq!(
			value,
			json!([
				{
					"type": "function",
					"name": "get_weather",
					"description": "Get the weather",
					"parameters": { "type": "object", "properties": { "location": { "type": "string" } } },
				},
			])
		);
	}

	#[test]
	fn test_otel_capture_content_default_off() {
		// No env var set in the test process → content capture is off by default.
		assert!(!capture_content());
	}
}
