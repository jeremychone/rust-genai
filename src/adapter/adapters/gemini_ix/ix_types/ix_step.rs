use crate::chat::{Binary, ContentPart, MessageContent, ToolCall, ToolResponse};
use serde::Deserialize;
use serde_json::Value;

/// A content block inside a `user_input` / `model_output` step, or a `thought` summary.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Content>
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IxContent {
	Text {
		#[serde(default)]
		text: String,
	},
	Image(IxMedia),
	Audio(IxMedia),
	Video(IxMedia),
	Document(IxMedia),

	#[serde(other)]
	Other,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IxMedia {
	pub mime_type: Option<String>,
	pub data: Option<String>,
	pub uri: Option<String>,
}

impl IxContent {
	pub fn into_content_part(self) -> Option<ContentPart> {
		let media = match self {
			IxContent::Text { text } => return Some(ContentPart::Text(text)),
			IxContent::Image(media)
			| IxContent::Audio(media)
			| IxContent::Video(media)
			| IxContent::Document(media) => media,
			IxContent::Other => return None,
		};

		let IxMedia { mime_type, data, uri } = media;

		let Some(mime_type) = mime_type else {
			tracing::warn!("GeminiIx - media content block without a mime_type (skipping)");
			return None;
		};

		match (data, uri) {
			(Some(data), _) => Some(ContentPart::Binary(Binary::from_base64(mime_type, data, None))),
			(None, Some(uri)) => Some(ContentPart::Binary(Binary::from_url(mime_type, uri, None))),
			(None, None) => {
				tracing::warn!("GeminiIx - media content block with neither `data` nor `uri` (skipping)");
				None
			}
		}
	}
}

/// One step of the interaction timeline.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Step>
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IxStep {
	UserInput,

	ModelOutput {
		#[serde(default)]
		content: Vec<IxContent>,
	},

	Thought {
		#[serde(default)]
		signature: Option<String>,
		#[serde(default)]
		summary: Vec<IxContent>,
	},

	FunctionCall {
		#[serde(default)]
		id: String,
		#[serde(default)]
		name: String,
		#[serde(default)]
		arguments: Value,
	},

	FunctionResult {
		#[serde(default)]
		call_id: String,
		#[serde(default)]
		name: Option<String>,
		#[serde(default)]
		result: Value,
	},

	#[serde(other)]
	Other,
}

impl IxStep {
	pub fn into_content_parts(self, reasoning: &mut String) -> Vec<ContentPart> {
		match self {
			IxStep::ModelOutput { content } => content.into_iter().filter_map(IxContent::into_content_part).collect(),

			IxStep::Thought { signature, summary } => {
				for part in summary {
					if let Some(ContentPart::Text(text)) = part.into_content_part() {
						reasoning.push_str(&text);
					}
				}
				match signature {
					Some(signature) if !signature.is_empty() => vec![ContentPart::ThoughtSignature(signature)],
					_ => Vec::new(),
				}
			}

			IxStep::FunctionCall { id, name, arguments } => vec![ContentPart::ToolCall(ToolCall {
				call_id: id,
				fn_name: name,
				fn_arguments: arguments,
				thought_signatures: None,
			})],

			IxStep::FunctionResult { call_id, name, result } => vec![ContentPart::ToolResponse(ToolResponse {
				call_id,
				fn_name: name,
				content: ix_result_to_content(result),
			})],

			IxStep::UserInput | IxStep::Other => Vec::new(),
		}
	}
}

/// `function_result.result` is `array (Content) | object | string`. The array form keeps its text
/// and media blocks (consecutive text blocks are concatenated), a string is kept as text, and any
/// other value is JSON-encoded as text.
fn ix_result_to_content(result: Value) -> MessageContent {
	match result {
		Value::String(text) => MessageContent::from_text(text),
		Value::Array(blocks) => {
			let mut content = MessageContent::default();
			let mut text_acc: Option<String> = None;
			for block in blocks.iter().cloned() {
				let Some(part) = serde_json::from_value::<IxContent>(block)
					.ok()
					.and_then(IxContent::into_content_part)
				else {
					continue;
				};
				match part {
					ContentPart::Text(text) => text_acc.get_or_insert_with(String::new).push_str(&text),
					other => {
						if let Some(text) = text_acc.take() {
							content.push(ContentPart::Text(text));
						}
						content.push(other);
					}
				}
			}
			if let Some(text) = text_acc {
				content.push(ContentPart::Text(text));
			}
			if content.is_empty() {
				MessageContent::from_text(Value::Array(blocks).to_string())
			} else {
				content
			}
		}
		other => MessageContent::from_text(other.to_string()),
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::json;

	#[test]
	fn test_function_result_keeps_text_and_media_blocks() -> serde_json::Result<()> {
		// -- Setup & Fixtures
		let step: IxStep = serde_json::from_value(json!({
			"type": "function_result",
			"call_id": "call_1",
			"name": "screenshot",
			"result": [
				{"type": "text", "text": "Screen"},
				{"type": "text", "text": "shot"},
				{"type": "image", "mime_type": "image/png", "data": "AAAA"},
			],
		}))?;

		// -- Exec
		let parts = step.into_content_parts(&mut String::new());

		// -- Check
		let [ContentPart::ToolResponse(tool_response)] = parts.as_slice() else {
			panic!("expected a single tool response, got {parts:?}");
		};
		assert_eq!(tool_response.text_content(), "Screenshot");
		assert_eq!(tool_response.binaries().len(), 1);
		assert_eq!(tool_response.binaries()[0].content_type, "image/png");
		Ok(())
	}
}

// endregion: --- Tests
