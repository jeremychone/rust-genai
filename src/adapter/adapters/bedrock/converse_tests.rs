type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;
use crate::chat::ChatMessage;

/// Converse natively supports image blocks inside `toolResult.content`, so a
/// `ToolResponse` with an image part emits the text block followed by the image block.
#[test]
fn test_bedrock_tool_response_image_part_serializes_tool_result_blocks() -> Result<()> {
	// -- Setup & Fixtures
	let tool_response =
		ToolResponse::new("call_1", "screenshot taken").with_parts([Binary::from_base64("image/png", "PNG64", None)]);
	let chat_req = ChatRequest::new(vec![ChatMessage::from(tool_response)]);

	// -- Exec
	let ConverseRequestParts { messages, .. } = into_converse_request_parts(chat_req)?;

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
	let ConverseRequestParts { messages, .. } = into_converse_request_parts(chat_req)?;

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
