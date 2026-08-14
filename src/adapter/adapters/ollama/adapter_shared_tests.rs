type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::chat::{ChatMessage, MessageContent};

/// Tool-result images cannot ride inside an Ollama `tool` message: the tool message
/// keeps its text (or the placeholder), and the images are carried by a follow-up
/// `user` message with the base64 `images` array.
#[test]
fn test_ollama_tool_response_image_parts_ride_in_followup_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(chat_req)?;

	// -- Check
	assert_eq!(messages.len(), 2, "tool message + follow-up user image message");
	assert_eq!(
		messages[0],
		json!({"role": "tool", "content": "(see attached image)"}),
		"image-only tool response must use the placeholder text"
	);
	assert_eq!(
		messages[1],
		json!({
			"role": "user",
			"content": "Attached image(s) from tool result:",
			"images": ["PNG64"],
		})
	);

	Ok(())
}

/// The `"(see attached image)"` placeholder must track the images contributed by the
/// CURRENT tool response only: with two responses in one message where the first
/// contributes an image and the second is empty with only skipped parts, the second's
/// text must be `"(no tool output)"` (not `"(see attached image)"`), even though the
/// message-level image accumulator is non-empty from the first response.
#[test]
fn test_ollama_tool_response_placeholder_tracks_own_images_only() -> Result<()> {
	// -- Setup & Fixtures
	let first = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	// URL-based image parts are skipped by the Ollama native adapter, so this
	// response contributes no usable image of its own.
	let second = ToolResponse::new("call_2", "").with_parts([Binary::from_url(
		"image/png",
		"https://example.com/shot.png",
		None,
	)]);
	let tool_msg = ChatMessage::tool(MessageContent::from_parts(vec![
		ContentPart::ToolResponse(first),
		ContentPart::ToolResponse(second),
	]));
	let chat_req = ChatRequest::new(vec![tool_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(chat_req)?;

	// -- Check
	assert_eq!(messages.len(), 2, "tool message + follow-up user image message");
	assert_eq!(
		messages[0]["content"],
		json!("(no tool output)"),
		"a response whose own parts were all skipped must not claim an attached image"
	);
	assert_eq!(
		messages[1],
		json!({
			"role": "user",
			"content": "Attached image(s) from tool result:",
			"images": ["PNG64"],
		}),
		"the first response's image still rides in the follow-up user message"
	);

	Ok(())
}

/// Regression guard: a text-only `ToolResponse` keeps its legacy shape with no
/// follow-up user message.
#[test]
fn test_ollama_tool_response_text_only_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(chat_req)?;

	// -- Check
	assert_eq!(messages, vec![json!({"role": "tool", "content": "42"})]);

	Ok(())
}
