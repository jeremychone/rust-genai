//! This is support implementation of the Ollama Adapter which can also be called by other Ollama Adapter Variants

use super::OllamaAdapter;
use crate::Headers;
use crate::adapter::AdapterKind;
use crate::chat::{Binary, BinarySource, ChatRequest, ContentPart, PromptTokensDetails, Tool, ToolName, Usage};
use crate::resolver::Endpoint;
use crate::webc::WebClient;
use crate::{Error, Result};
use serde_json::{Value, json};
use value_ext::JsonValueExt;

/// Support functions for other adapters that share Ollama APIs
impl OllamaAdapter {
	pub(in crate::adapter::adapters) async fn list_model_names(
		adapter_kind: AdapterKind,
		endpoint: Endpoint,
		headers: Headers,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		let base_url = endpoint.base_url();
		let url = format!("{base_url}api/tags");

		let mut res = web_client
			.do_get(&url, &headers)
			.await
			.map_err(|webc_error| Error::WebAdapterCall {
				adapter_kind,
				webc_error,
			})?;

		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("name")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}

	pub(in crate::adapter::adapters) fn into_usage(body: &mut Value) -> Usage {
		let prompt_tokens = body.x_take::<i32>("prompt_eval_count").ok();
		// `prompt_eval_cached_count` is the subset of the prompt tokens served
		// from the KV cache; `prompt_eval_count` remains the full input count.
		// Ref: https://docs.ollama.com/api/usage
		let cached_tokens = body.x_take::<i32>("prompt_eval_cached_count").ok().filter(|v| *v > 0);
		let completion_tokens = body.x_take::<i32>("eval_count").ok();
		let total_tokens = match (prompt_tokens, completion_tokens) {
			(Some(p), Some(c)) => Some(p + c),
			_ => None,
		};

		Usage {
			prompt_tokens,
			prompt_tokens_details: cached_tokens.map(|n| PromptTokensDetails {
				cached_tokens: Some(n),
				..Default::default()
			}),
			completion_tokens,
			total_tokens,
			..Default::default()
		}
	}

	/// Takes the GenAI ChatMessages and constructs the JSON Messages for Ollama.
	pub(in crate::adapter::adapters) fn into_ollama_request_parts(chat_req: ChatRequest) -> Result<OllamaRequestParts> {
		let mut messages = Vec::new();

		// -- System
		if let Some(system) = chat_req.system {
			messages.push(json!({
				"role": "system",
				"content": system,
			}));
		}

		// -- Messages
		for msg in chat_req.messages {
			let mut ollama_msg = json!({
				"role": msg.role.to_string().to_lowercase(),
			});

			let mut content = String::new();
			let mut images = Vec::new();
			let mut tool_calls = Vec::new();

			for part in msg.content {
				match part {
					ContentPart::Text(txt) => content.push_str(&txt),
					ContentPart::Binary(Binary {
						content_type,
						source: BinarySource::Base64(data),
						..
					}) if content_type.starts_with("image/") => {
						// Note: Ollama native API expects images in base64 format in a field named "images" as an array.
						images.push(data);
					}
					ContentPart::ToolCall(tool_call) => {
						tool_calls.push(json!({
							"function": {
								"name": tool_call.fn_name,
								"arguments": tool_call.fn_arguments,
							}
						}));
					}
					ContentPart::ToolResponse(tr) => {
						// Note: Ollama native API expects role "tool" for tool response
						ollama_msg.x_insert("content", tr.content)?;
					}
					_ => {}
				}
			}

			if !content.is_empty() {
				ollama_msg.x_insert("content", content)?;
			}
			if !images.is_empty() {
				ollama_msg.x_insert("images", images)?;
			}
			if !tool_calls.is_empty() {
				ollama_msg.x_insert("tool_calls", tool_calls)?;
			}

			messages.push(ollama_msg);
		}

		// -- Tools
		let tools = chat_req
			.tools
			.map(|tools| tools.into_iter().map(Self::tool_to_ollama_tool).collect::<Result<Vec<Value>>>())
			.transpose()?;

		Ok(OllamaRequestParts { messages, tools })
	}

	pub(in crate::adapter::adapters) fn tool_to_ollama_tool(tool: Tool) -> Result<Value> {
		let Tool {
			name,
			description,
			schema,
			..
		} = tool;

		let name = match name {
			ToolName::WebSearch => "web_search".to_string(),
			ToolName::Custom(name) => name,
		};

		let mut tool_value = json!({
			"type": "function",
			"function": {
				"name": name,
			}
		});

		if let Some(description) = description {
			tool_value.x_insert("/function/description", description)?;
		}
		if let Some(parameters) = schema {
			tool_value.x_insert("/function/parameters", parameters)?;
		}

		Ok(tool_value)
	}
}

pub(in crate::adapter::adapters) struct OllamaRequestParts {
	pub messages: Vec<Value>,
	pub tools: Option<Vec<Value>>,
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_into_usage_with_cached_tokens() {
		// -- Setup & Fixtures
		let mut body = json!({
			"model": "qwen3",
			"done": true,
			"done_reason": "stop",
			"prompt_eval_count": 21,
			"prompt_eval_cached_count": 17,
			"eval_count": 40
		});

		// -- Exec
		let usage = OllamaAdapter::into_usage(&mut body);

		// -- Check
		assert_eq!(
			usage.prompt_tokens,
			Some(21),
			"prompt_eval_count is the full input count"
		);
		assert_eq!(usage.completion_tokens, Some(40));
		assert_eq!(usage.total_tokens, Some(61));
		let details = usage.prompt_tokens_details.expect("details should be set");
		assert_eq!(details.cached_tokens, Some(17));
		assert_eq!(details.cache_creation_tokens, None);
		assert!(details.cache_creation_details.is_none());
		assert_eq!(details.audio_tokens, None);
	}

	#[test]
	fn test_into_usage_without_cached_field_has_no_details() {
		// -- Setup & Fixtures
		let mut body = json!({
			"model": "qwen3",
			"done": true,
			"prompt_eval_count": 21,
			"eval_count": 40
		});

		// -- Exec
		let usage = OllamaAdapter::into_usage(&mut body);

		// -- Check
		assert_eq!(usage.prompt_tokens, Some(21));
		assert!(
			usage.prompt_tokens_details.is_none(),
			"no cached field means no details"
		);
	}

	#[test]
	fn test_into_usage_zero_cached_tokens_yields_no_details() {
		// -- Setup & Fixtures
		// A cached count of 0 is normalized away (zero-as-none), like other adapters.
		let mut body = json!({
			"model": "qwen3",
			"done": true,
			"prompt_eval_count": 21,
			"prompt_eval_cached_count": 0,
			"eval_count": 40
		});

		// -- Exec
		let usage = OllamaAdapter::into_usage(&mut body);

		// -- Check
		assert_eq!(usage.prompt_tokens, Some(21));
		assert!(
			usage.prompt_tokens_details.is_none(),
			"cached count 0 must not produce details"
		);
	}
}

// End of Tests
