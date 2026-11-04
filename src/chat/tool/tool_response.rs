use super::ToolCall;
use crate::chat::{Binary, ContentPart, MessageContent};
use serde::{Deserialize, Deserializer, Serialize};
use serde_with::skip_serializing_none;

/// Response produced by a tool invocation, paired with the originating tool call ID.
///
/// The content can carry text and binary parts (e.g., an image or a PDF produced by the tool).
/// Binary parts are sent natively by the adapters whose APIs accept them in tool results
/// (Anthropic, Bedrock Converse, OpenAI Responses, Gemini 3+, Gemini Interactions, Ollama images).
/// Other adapters send only the text parts and drop the binaries with a warning.
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
	/// Identifier of the originating tool call.
	pub call_id: String,
	/// Name of the function/tool that produced this response.
	///
	/// Most providers correlate responses by call ID, but Gemini's
	/// `functionResponse.name` expects the function name.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub fn_name: Option<String>,
	/// Tool output payload. Text parts are often JSON-serialized content.
	///
	/// Only `Text` and `Binary` parts are meaningful here; adapters ignore other part kinds.
	/// Deserialization also accepts a plain string (the format used before binary support).
	#[serde(deserialize_with = "deserialize_tool_response_content")]
	pub content: MessageContent,
}

/// Constructor
impl ToolResponse {
	/// Creates a new ToolResponse with the provided tool_call_id and content.
	pub fn new(tool_call_id: impl Into<String>, content: impl Into<MessageContent>) -> Self {
		Self {
			call_id: tool_call_id.into(),
			fn_name: None,
			content: content.into(),
		}
	}

	/// Creates a ToolResponse from the originating ToolCall.
	pub fn from_tool_call(tool_call: &ToolCall, content: impl Into<MessageContent>) -> Self {
		Self {
			call_id: tool_call.call_id.clone(),
			fn_name: Some(tool_call.fn_name.clone()),
			content: content.into(),
		}
	}

	/// Attach the function/tool name to this response.
	pub fn with_fn_name(mut self, fn_name: impl Into<String>) -> Self {
		self.fn_name = Some(fn_name.into());
		self
	}

	/// Append a text part to this response.
	pub fn with_text(mut self, text: impl Into<String>) -> Self {
		self.content.push(ContentPart::Text(text.into()));
		self
	}

	/// Append a binary part (e.g., an image or a PDF) to this response.
	pub fn with_binary(mut self, binary: Binary) -> Self {
		self.content.push(ContentPart::Binary(binary));
		self
	}
}

/// Computed accessors
impl ToolResponse {
	/// Returns an approximate in-memory size of this `ToolResponse`, in bytes,
	/// computed as the sum of the UTF-8 lengths of:
	/// - `call_id`
	/// - `fn_name` (if any)
	/// - `content` (see `MessageContent::size()`)
	pub fn size(&self) -> usize {
		self.call_id.len() + self.fn_name.as_ref().map(|name| name.len()).unwrap_or(0) + self.content.size()
	}

	/// All text parts joined with a blank line (empty string when there is no text).
	///
	/// This is what text-only providers receive.
	pub fn text_content(&self) -> String {
		self.content.joined_texts().unwrap_or_default()
	}

	/// References to the binary parts of this response.
	pub fn binaries(&self) -> Vec<&Binary> {
		self.content.binaries()
	}

	/// True if this response carries at least one binary part.
	pub fn contains_binary(&self) -> bool {
		self.content.contains_binary()
	}
}

/// Getters
impl ToolResponse {
	/// The originating tool call ID.
	pub fn tool_call_id(&self) -> &str {
		&self.call_id
	}

	/// The function/tool name, if set.
	pub fn fn_name(&self) -> Option<&str> {
		self.fn_name.as_deref()
	}

	/// The response content.
	pub fn content(&self) -> &MessageContent {
		&self.content
	}
}

// region:    --- Serde Support

fn deserialize_tool_response_content<'de, D>(deserializer: D) -> Result<MessageContent, D::Error>
where
	D: Deserializer<'de>,
{
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum ContentRepr {
		Text(String),
		Parts(MessageContent),
	}

	Ok(match ContentRepr::deserialize(deserializer)? {
		ContentRepr::Text(text) => MessageContent::from_text(text),
		ContentRepr::Parts(content) => content,
	})
}

// endregion: --- Serde Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_tool_response_deserializes_legacy_string_content() -> serde_json::Result<()> {
		let tool_response: ToolResponse = serde_json::from_str(r#"{"call_id":"call_1","content":"25 C"}"#)?;

		assert_eq!(tool_response.text_content(), "25 C");
		assert!(!tool_response.contains_binary());
		Ok(())
	}

	#[test]
	fn test_tool_response_serde_roundtrip_with_binary() -> serde_json::Result<()> {
		let tool_response =
			ToolResponse::new("call_1", "Screenshot").with_binary(Binary::from_base64("image/png", "AAAA", None));

		let json = serde_json::to_string(&tool_response)?;
		let back: ToolResponse = serde_json::from_str(&json)?;

		assert_eq!(back.text_content(), "Screenshot");
		assert_eq!(back.binaries().len(), 1);
		assert_eq!(back.binaries()[0].content_type, "image/png");
		Ok(())
	}
}

// endregion: --- Tests
