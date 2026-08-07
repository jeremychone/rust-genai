type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::adapter::AdapterKind;
use crate::chat::ChatMessage;

fn test_model_iden() -> ModelIden {
	ModelIden::new(AdapterKind::BedrockApi, "anthropic.claude-haiku-4-5")
}

/// Converse natively supports image blocks inside `toolResult.content`, so a
/// `ToolResponse` with an image part emits the text block followed by the image block.
#[test]
fn test_bedrock_tool_response_image_part_serializes_tool_result_blocks() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response =
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let ConverseRequestParts { messages, .. } = into_converse_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![json!({
			"role": "user",
			"content": [{
				"toolResult": {
					"toolUseId": "call_1",
					"content": [
						{ "text": "screenshot taken" },
						{ "image": { "format": "png", "source": { "bytes": "PNG64" } } },
					],
				}
			}]
		})]
	);

	Ok(())
}

/// Regression guard: a text-only `ToolResponse` keeps the legacy single text block.
#[test]
fn test_bedrock_tool_response_text_only_serializes_as_before() -> Result<()> {
	// -- Setup & Fixtures
	let chat_req = ChatRequest::new(vec![ChatMessage::from(ToolResponse::new("call_1", "42"))]);

	// -- Exec
	let ConverseRequestParts { messages, .. } = into_converse_request_parts(&test_model_iden(), chat_req)?;

	// -- Check
	assert_eq!(
		messages,
		vec![json!({
			"role": "user",
			"content": [{
				"toolResult": {
					"toolUseId": "call_1",
					"content": [{ "text": "42" }],
				}
			}]
		})]
	);

	Ok(())
}

/// A `ToolResponse` embedded in an Assistant message has no representation on any
/// provider wire (there is no "tool result authored by the assistant"), so the
/// serializer must reject the shape with a hard error instead of silently dropping
/// the content (the previous behavior).
#[test]
fn test_bedrock_assistant_embedded_tool_response_is_rejected() -> Result<()> {
	// -- Setup & Fixtures
	let assistant_msg = ChatMessage::assistant(MessageContent::from_parts(vec![
		ContentPart::from_text("checking"),
		ContentPart::ToolCall(ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "get_weather".to_string(),
			fn_arguments: json!({"city": "Paris"}),
			thought_signatures: None,
		}),
		ContentPart::ToolResponse(ToolResponse::new("call_1", "sunny")),
	]));
	let chat_req = ChatRequest::new(vec![ChatMessage::user("weather?"), assistant_msg]);

	// -- Exec
	let err = into_converse_request_parts(&test_model_iden(), chat_req)
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
