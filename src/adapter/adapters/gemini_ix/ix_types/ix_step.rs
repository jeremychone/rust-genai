use crate::chat::{Binary, ContentPart, ToolCall, ToolResponse};
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
			tracing::warn!("GeminiInteractions - media content block without a mime_type (skipping)");
			return None;
		};

		match (data, uri) {
			(Some(data), _) => Some(ContentPart::Binary(Binary::from_base64(mime_type, data, None))),
			(None, Some(uri)) => Some(ContentPart::Binary(Binary::from_url(mime_type, uri, None))),
			(None, None) => {
				tracing::warn!("GeminiInteractions - media content block with neither `data` nor `uri` (skipping)");
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
				content: ix_result_to_string(result),
			})],

			IxStep::UserInput | IxStep::Other => Vec::new(),
		}
	}
}

/// `function_result.result` is `array (Content) | object | string`. `ToolResponse.content` is a
/// plain string, so collapse the array form to its concatenated text and JSON-encode the rest.
fn ix_result_to_string(result: Value) -> String {
	match result {
		Value::String(text) => text,
		Value::Array(blocks) => {
			let texts: Vec<String> = blocks
				.iter()
				.filter_map(|block| block.get("text").and_then(Value::as_str).map(String::from))
				.collect();
			if texts.is_empty() {
				Value::Array(blocks).to_string()
			} else {
				texts.join("")
			}
		}
		other => other.to_string(),
	}
}
