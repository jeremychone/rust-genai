use crate::chat::{ContentPart, ToolCall};
use crate::{Error, Result};
use serde_json::Value;
use value_ext::JsonValueExt;

/// Convert a OpenAI response output Item to a ContentPart
///
/// NOTE: At this point this is infallible, will ignore item that cannot be transformed
impl ContentPart {
	pub fn from_resp_output_item(mut item_value: Value) -> Result<Vec<Self>> {
		let mut parts = Vec::new();
		let Some(item_type) = ItemType::from_item_value(&item_value) else {
			return Ok(parts);
		};

		match item_type {
			ItemType::Message => {
				if let Ok(content) = item_value.x_remove::<Vec<Value>>("content") {
					// each content item {}
					for mut content_item in content {
						if let Ok("output_text") = content_item.x_get_str("type")
							&& let Ok(text) = content_item.x_remove::<String>("text")
						{
							parts.push(text.into())
						}
					}
				}
			}
			ItemType::FunctionCall => {
				let fn_name = item_value.x_remove::<String>("name")?;
				let call_id = item_value.x_remove::<String>("call_id")?;
				let arguments = item_value.x_remove::<String>("arguments")?;
				let fn_arguments: Value =
					serde_json::from_str(&arguments).map_err(|_| Error::InvalidJsonResponseElement {
						info: "tool call arguments is not an object.\nCause",
					})?;

				let tool_call = ToolCall {
					call_id,
					fn_name,
					fn_arguments,
					thought_signatures: None,
				};

				parts.push(tool_call.into());
			}
			ItemType::CustomToolCall => {
				let fn_name = item_value.x_remove::<String>("name")?;
				let call_id = item_value.x_remove::<String>("call_id")?;
				let input = item_value.x_remove::<String>("input")?;
				parts.push(
					ToolCall {
						call_id,
						fn_name,
						fn_arguments: Value::String(input),
						thought_signatures: None,
					}
					.into(),
				);
			}
		}

		Ok(parts)
	}
}

// region:    --- Support Type

/// The managed
enum ItemType {
	Message,
	FunctionCall,
	CustomToolCall,
}

impl ItemType {
	fn from_item_value(item_value: &Value) -> Option<Self> {
		let typ = item_value.x_get_str("type").ok()?;
		match typ {
			"message" => Some(ItemType::Message),
			"function_call" => Some(ItemType::FunctionCall),
			"custom_tool_call" => Some(ItemType::CustomToolCall),
			_ => None,
		}
	}
}

// endregion: --- Support Type

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decodes_custom_tool_call_as_raw_string_input() {
		let parts = ContentPart::from_resp_output_item(serde_json::json!({
			"type": "custom_tool_call",
			"call_id": "call_patch",
			"name": "apply_patch",
			"input": "*** Begin Patch\n*** End Patch\n",
		}))
		.unwrap();
		let ContentPart::ToolCall(call) = &parts[0] else {
			panic!("expected a tool call");
		};
		assert_eq!(call.call_id, "call_patch");
		assert_eq!(call.fn_name, "apply_patch");
		assert_eq!(
			call.fn_arguments,
			Value::String("*** Begin Patch\n*** End Patch\n".to_string())
		);
	}
}
