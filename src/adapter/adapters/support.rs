//! This support module is for common constructs and utilities for all the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::ModelIden;
use crate::chat::{ChatOptionsSet, Usage};
use crate::resolver::AuthData;
use crate::{Error, Result};

pub fn get_api_key(auth: AuthData, model: &ModelIden) -> Result<String> {
	auth.single_key_value().map_err(|resolver_error| Error::Resolver {
		model_iden: model.clone(),
		resolver_error,
	})
}

/// Build the error for a `ContentPart::ToolResponse` embedded in an Assistant-role message.
///
/// No provider wire has a representation for a tool result authored by the assistant
/// (tool results are standalone `role:"tool"` messages / output items on the OpenAI
/// wires, and user-carried `tool_result` / `toolResult` / `functionResponse` blocks on
/// the Anthropic-style wires), so every serializer rejects the shape with this same
/// error instead of silently dropping the content or inventing a placement the wire
/// does not define. The supported shape is a Tool-role message.
pub fn assistant_embedded_tool_response_err(model_iden: &ModelIden) -> Error {
	Error::MessageContentTypeNotSupported {
		model_iden: model_iden.clone(),
		cause: "ContentPart::ToolResponse is not supported in an Assistant-role message — no provider wire represents a tool result authored by the assistant. Send the tool response as a Tool-role message instead (e.g., `ChatMessage::from(ToolResponse)`)",
	}
}

// region:    --- Tool Response Binary Parts

/// Leading text of the follow-up `user` message that carries tool-result images on
/// wire formats that cannot express images inside the tool-result item itself
/// (e.g., OpenAI Chat Completions `tool` messages, Gemini `functionResponse`, Ollama).
pub const TOOL_RESULT_IMAGES_LABEL: &str = "Attached image(s) from tool result:";

/// Resolve the text content of a tool-result message when the `ToolResponse`
/// carries binary parts.
///
/// - Non-empty text content is kept as-is.
/// - Empty text with image parts becomes the `"(see attached image)"` placeholder,
///   pointing the model at the follow-up user message that carries the images.
/// - Empty text without any usable image part becomes `"(no tool output)"`.
///
/// NOTE: Only called when `ToolResponse.parts` is present, so plain text-only
///       responses keep their exact legacy serialization.
pub fn tool_response_fallback_text(content: String, has_images: bool) -> String {
	if !content.is_empty() {
		content
	} else if has_images {
		"(see attached image)".to_string()
	} else {
		"(no tool output)".to_string()
	}
}

// endregion: --- Tool Response Binary Parts

// region:    --- StreamerChatOptions

#[derive(Debug)]
pub struct StreamerOptions {
	pub capture_usage: bool,
	pub capture_reasoning_content: bool,
	pub capture_content: bool,
	pub capture_tool_calls: bool,
	pub model_iden: ModelIden,
}

impl StreamerOptions {
	pub fn new(model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			capture_usage: options_set.capture_usage().unwrap_or(false),
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_reasoning_content: options_set.capture_reasoning_content().unwrap_or(false),
			capture_tool_calls: options_set.capture_tool_calls().unwrap_or(false),
			model_iden,
		}
	}
}

// endregion: --- StreamerChatOptions

// region:    --- Streamer Captured Data

#[derive(Debug, Default)]
pub struct StreamerCapturedData {
	pub usage: Option<Usage>,
	pub stop_reason: Option<String>,
	pub content: Option<String>,
	pub reasoning_content: Option<String>,
	pub tool_calls: Option<Vec<crate::chat::ToolCall>>,
	pub thought_signatures: Option<Vec<String>>,
}

// endregion: --- Streamer Captured Data
