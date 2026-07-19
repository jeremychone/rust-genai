use crate::chat::{ChatOptionsSet, ChatResponseFormat, JsonSchemaDialect, sanitize_json_schema};
use serde_json::Value;

pub(crate) enum OpenAiResponseFormatPlan {
	None,
	JsonMode,
	JsonSchema { name: String, schema: Value },
}

pub(crate) fn response_format_plan(options: &ChatOptionsSet<'_, '_>) -> OpenAiResponseFormatPlan {
	match options.response_format() {
		None => OpenAiResponseFormatPlan::None,
		Some(ChatResponseFormat::JsonMode) => OpenAiResponseFormatPlan::JsonMode,
		Some(ChatResponseFormat::JsonSpec(spec)) => OpenAiResponseFormatPlan::JsonSchema {
			name: spec.name.clone(),
			schema: sanitize_json_schema(&spec.schema, JsonSchemaDialect::OpenAiStrict),
		},
	}
}

pub(crate) fn tool_parameters_schema(schema: Option<Value>, strict: bool) -> Option<Value> {
	if strict {
		schema.map(|schema| sanitize_json_schema(&schema, JsonSchemaDialect::OpenAiStrictTool))
	} else {
		schema
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chat::{ChatOptions, JsonSpec};
	use serde_json::json;

	#[test]
	fn unsupported_schema_is_still_sent_to_the_backend() {
		let options = ChatOptions::default().with_response_format(JsonSpec::new(
			"array",
			json!({"type": "array", "items": {"type": "integer"}}),
		));
		let options = ChatOptionsSet::default().with_chat_options(Some(&options));
		let plan = response_format_plan(&options);
		let OpenAiResponseFormatPlan::JsonSchema { schema, .. } = plan else {
			panic!("array schema should be sent for backend validation");
		};
		assert_eq!(schema, json!({"type": "array", "items": {"type": "integer"}}));
	}

	#[test]
	fn strict_tool_schema_uses_the_openai_tool_dialect() {
		let schema = json!({
			"type": "object",
			"properties": {
				"required_name": {"type": "string"},
				"optional_limit": {"type": "integer", "default": 10}
			},
			"required": ["required_name"]
		});
		let result = tool_parameters_schema(Some(schema), true).unwrap();
		assert_eq!(result["required"], json!(["required_name", "optional_limit"]));
		assert_eq!(result["properties"]["optional_limit"]["default"], json!(10));
		assert_eq!(result["additionalProperties"], json!(false));
	}

	#[test]
	fn non_strict_tool_schema_is_untouched() {
		let schema = json!({
			"type": "object",
			"properties": {"optional_limit": {"type": "integer", "default": 10}}
		});
		assert_eq!(tool_parameters_schema(Some(schema.clone()), false), Some(schema));
	}
}
