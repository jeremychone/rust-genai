type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::chat::{ChatMessage, MessageContent};

fn test_model_iden() -> ModelIden {
	ModelIden::new(AdapterKind::Ollama, "llama3.2")
}

/// Tool-result images cannot ride inside an Ollama `tool` message: the tool message
/// keeps its text (or the placeholder), and the images are carried by a follow-up
/// `user` message with the base64 `images` array.
#[test]
fn test_ollama_tool_response_image_parts_ride_in_followup_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

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
///
/// NOTE: this test previously pinned the old single-tool-message shape, where each
/// response's content overwrote the previous in the carrying message and only the
/// last survived. That overwrite was a bug, and the expectation here changed
/// deliberately with the fix: a Tool-role message carrying multiple `ToolResponse`
/// parts now emits one `role:"tool"` message per response, in part order.
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
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages.len(),
		3,
		"one tool message per response + follow-up user image message"
	);
	assert_eq!(
		messages[0]["content"],
		json!("(see attached image)"),
		"the first response contributed its own image, so it claims the placeholder"
	);
	assert_eq!(
		messages[1]["content"],
		json!("(no tool output)"),
		"a response whose own parts were all skipped must not claim an attached image"
	);
	assert_eq!(
		messages[2],
		json!({
			"role": "user",
			"content": "Attached image(s) from tool result:",
			"images": ["PNG64"],
		}),
		"the first response's image still rides in the follow-up user message"
	);

	Ok(())
}

/// A Tool-role message carrying multiple `ToolResponse` parts (the
/// `ChatMessage::from(Vec<ToolResponse>)` shape) emits ONE `role:"tool"` message PER
/// response, in part order, matching the per-response messages the other serializers
/// emit (previously each response's content overwrote the previous in a single tool
/// message, so only the last response's text survived).
#[test]
fn test_ollama_tool_role_multiple_responses_emit_one_tool_message_each() -> Result<()> {
	// -- Setup & Fixtures
	let tool_msg = ChatMessage::from(vec![
		ToolResponse::new("call_1", "one"),
		ToolResponse::new("call_2", "two"),
	]);
	let chat_req = ChatRequest::new(vec![tool_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			json!({"role": "tool", "content": "one"}),
			json!({"role": "tool", "content": "two"}),
		]
	);

	Ok(())
}

/// Images from MULTIPLE `ToolResponse` parts of one Tool-role message accumulate, in
/// part order, into the SINGLE labeled follow-up user image message (not one
/// follow-up per response), emitted after the per-response tool messages.
#[test]
fn test_ollama_tool_role_multiple_responses_images_accumulate_in_one_followup() -> Result<()> {
	// -- Setup & Fixtures
	let first = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "A64", None)]);
	let second = ToolResponse::new("call_2", "done").with_parts([Binary::from_base64("image/png", "B64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(vec![first, second])]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			json!({"role": "tool", "content": "(see attached image)"}),
			json!({"role": "tool", "content": "done"}),
			json!({
				"role": "user",
				"content": "Attached image(s) from tool result:",
				"images": ["A64", "B64"],
			}),
		]
	);

	Ok(())
}

/// Regression guard: a text-only `ToolResponse` keeps its legacy shape with no
/// follow-up user message — byte-identical on the wire, pinned via the serialized
/// string (`serde_json` runs with `preserve_order`, so key order is meaningful).
#[test]
fn test_ollama_tool_response_text_only_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(messages, vec![json!({"role": "tool", "content": "42"})]);
	assert_eq!(
		serde_json::to_string(&messages)?,
		r#"[{"role":"tool","content":"42"}]"#,
		"single-response wire bytes must stay identical"
	);

	Ok(())
}

/// A `ToolResponse` embedded in a User-role message (the Anthropic-style shape where
/// tool results ride as user content blocks) is extracted into a standalone
/// `role:"tool"` message emitted before the user message carrying the remaining
/// content (previously the response text was garbled into the user `content`, where
/// sibling text parts overwrote it).
#[test]
fn test_ollama_user_embedded_tool_response_extracted_before_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let user_msg = ChatMessage::user(MessageContent::from_parts(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
		ContentPart::from_text("thanks, now summarize"),
	]));
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			json!({"role": "tool", "content": "sunny"}),
			json!({"role": "user", "content": "thanks, now summarize"}),
		]
	);

	Ok(())
}

/// Multiple `ToolResponse` parts embedded in one user message are ALL extracted, in
/// part order, before the remaining user message.
#[test]
fn test_ollama_user_embedded_tool_responses_all_extracted_in_order() -> Result<()> {
	// -- Setup & Fixtures
	let user_msg = ChatMessage::user(MessageContent::from_parts(vec![
		ContentPart::ToolResponse(ToolResponse::new("call_1", "one")),
		ContentPart::from_text("both done"),
		ContentPart::ToolResponse(ToolResponse::new("call_2", "two")),
	]));
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			json!({"role": "tool", "content": "one"}),
			json!({"role": "tool", "content": "two"}),
			json!({"role": "user", "content": "both done"}),
		]
	);

	Ok(())
}

/// A user message carrying ONLY an embedded `ToolResponse` has nothing left to say
/// after the extraction, so the now-empty user message is omitted.
#[test]
fn test_ollama_user_embedded_tool_response_only_omits_empty_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let user_msg = ChatMessage::user(MessageContent::from_parts(vec![ContentPart::ToolResponse(
		ToolResponse::new("call_1", "sunny"),
	)]));
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(messages, vec![json!({"role": "tool", "content": "sunny"})]);

	Ok(())
}

/// Image parts of a user-embedded `ToolResponse` ride the same follow-up user image
/// message as Tool-role responses (label + base64 `images` array), emitted after the
/// remaining user message; the extracted tool message uses the placeholder text.
#[test]
fn test_ollama_user_embedded_tool_response_image_rides_followup_user_message() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response = ToolResponse::new("call_1", "").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let user_msg = ChatMessage::user(MessageContent::from_parts(vec![
		ContentPart::ToolResponse(tool_response),
		ContentPart::from_text("what is in the screenshot?"),
	]));
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			json!({"role": "tool", "content": "(see attached image)"}),
			json!({"role": "user", "content": "what is in the screenshot?"}),
			json!({
				"role": "user",
				"content": "Attached image(s) from tool result:",
				"images": ["PNG64"],
			}),
		]
	);

	Ok(())
}

/// Regression guard: a plain user message (text and image parts, no embedded tool
/// response) keeps its legacy single-message shape.
#[test]
fn test_ollama_plain_user_message_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let user_msg = ChatMessage::user(MessageContent::from_parts(vec![
		ContentPart::from_text("describe "),
		ContentPart::from_text("this"),
		ContentPart::Binary(Binary::from_base64("image/png", "IMG64", None)),
	]));
	let chat_req = ChatRequest::new(vec![user_msg]);

	// -- Exec
	let OllamaRequestParts { messages, .. } = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![json!({"role": "user", "content": "describe this", "images": ["IMG64"]})]
	);

	Ok(())
}

/// A `ToolResponse` embedded in an Assistant message has no representation on any
/// provider wire (there is no "tool result authored by the assistant"), so the
/// serializer must reject the shape with a hard error instead of garbling it into
/// assistant content (the previous behavior).
#[test]
fn test_ollama_assistant_embedded_tool_response_is_rejected() -> Result<()> {
	// -- Setup & Fixtures
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![
		ContentPart::from_text("checking"),
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
	]));
	let chat_req = ChatRequest::new(vec![ChatMessage::user("weather?"), assistant_msg]);

	// -- Exec
	let err = OllamaAdapter::into_ollama_request_parts(&test_model_iden(), chat_req)
		.map(|_| ())
		.expect_err("assistant-embedded tool response must fail serialization");

	// -- Check
	let Error::MessageContentTypeNotSupported { cause, .. } = err else {
		return Err(format!("expected MessageContentTypeNotSupported, got: {err}").into());
	};
	assert!(
		cause.contains("Assistant-role message"),
		"cause must name the unsupported shape: {cause}"
	);
	assert!(
		cause.contains("Tool-role message"),
		"cause must point at the supported Tool-role shape: {cause}"
	);

	Ok(())
}
