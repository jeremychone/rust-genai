type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use serde_json::json;

/// A text-only ToolResponse must serialize exactly as before the `parts` addition
/// (no `parts` key), so persisted chat histories keep the same JSON shape.
#[test]
fn test_tool_response_text_only_serde_unchanged() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "42");

	// -- Exec
	let value = serde_json::to_value(&tool_response)?;

	// -- Check
	assert_eq!(value, json!({"call_id": "call_1", "content": "42"}));

	Ok(())
}

/// Legacy JSON (without `parts`) must still deserialize.
#[test]
fn test_tool_response_deserialize_legacy_json() -> Result<()> {
	// -- Setup & Fixtures
	let legacy_json = json!({"call_id": "call_1", "content": "42"});

	// -- Exec
	let tool_response: ToolResponse = serde_json::from_value(legacy_json)?;

	// -- Check
	assert_eq!(tool_response.call_id, "call_1");
	assert_eq!(tool_response.content, "42");
	assert!(tool_response.parts.is_none());

	Ok(())
}

/// `with_parts` and `append_binary` builders populate `parts`, and `parts`
/// round-trips through serde.
#[test]
fn test_tool_response_with_parts_serde_roundtrip() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "screenshot taken")
		.with_parts([Binary::from_base64("image/png", "AAA=", None)])
		.append_binary(Binary::from_base64("image/jpeg", "BBB=", Some("shot.jpg".to_string())));

	// -- Exec
	let value = serde_json::to_value(&tool_response)?;
	let back: ToolResponse = serde_json::from_value(value)?;

	// -- Check
	let parts = back.parts.ok_or("should have parts")?;
	assert_eq!(parts.len(), 2);
	assert_eq!(parts[0].content_type, "image/png");
	assert_eq!(parts[1].content_type, "image/jpeg");
	assert_eq!(parts[1].name.as_deref(), Some("shot.jpg"));

	Ok(())
}
