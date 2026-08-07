//! This is support implementation of the Ollama Adapter which can also be called by other Ollama Adapter Variants

use super::OllamaAdapter;
use crate::Headers;
use crate::adapter::AdapterKind;
use crate::adapter::adapters::support::{
	TOOL_RESULT_IMAGES_LABEL, assistant_embedded_tool_response_err, tool_response_fallback_text,
};
use crate::chat::{Binary, BinarySource, ChatRequest, ChatRole, ContentPart, Tool, ToolName, ToolResponse, Usage};
use crate::resolver::Endpoint;
use crate::webc::WebClient;
use crate::{Error, ModelIden, Result};
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::warn;
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
		let completion_tokens = body.x_take::<i32>("eval_count").ok();
		let total_tokens = match (prompt_tokens, completion_tokens) {
			(Some(p), Some(c)) => Some(p + c),
			_ => None,
		};

		Usage {
			prompt_tokens,
			completion_tokens,
			total_tokens,
			..Default::default()
		}
	}

	/// Takes the GenAI ChatMessages and constructs the JSON Messages for Ollama.
	pub(in crate::adapter::adapters) fn into_ollama_request_parts(
		model_iden: &ModelIden,
		chat_req: ChatRequest,
	) -> Result<OllamaRequestParts> {
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
			// Images attached to tool responses (`ToolResponse.parts`); they ride in a
			// follow-up "user" message since tool messages carry only text content.
			let mut tool_response_images = Vec::new();
			// Whether a `ToolResponse` part of this message was emitted as a standalone
			// `role:"tool"` message (see below); when nothing else remains, the carrying
			// message is omitted.
			let mut had_tool_responses = false;

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
						// No provider wire represents a tool result authored by the assistant;
						// fail loudly instead of garbling it into assistant content
						// (use a Tool-role message).
						if matches!(msg.role, ChatRole::Assistant) {
							return Err(assistant_embedded_tool_response_err(model_iden));
						}
						// Note: Ollama native API expects role "tool" for tool response, and
						// the standalone `role:"tool"` message is the wire's only tool-result
						// representation, so ONE such message is emitted PER response, in part
						// order — a Tool-role message can carry several (matching the
						// per-response messages the OpenAI Chat Completions serializer emits).
						// For a tool response embedded in a user message (the Anthropic-style
						// shape where tool results ride as user-message content blocks), the
						// extracted tool message is emitted BEFORE the remaining user message.
						// Images of every response in the message ride the same single
						// labeled follow-up user image message.
						let tr_content = tool_response_content_text(tr, &mut tool_response_images);
						had_tool_responses = true;
						messages.push(json!({
							"role": "tool",
							"content": tr_content,
						}));
					}
					_ => {}
				}
			}

			let leftover_is_empty = content.is_empty() && images.is_empty() && tool_calls.is_empty();
			if !content.is_empty() {
				ollama_msg.x_insert("content", content)?;
			}
			if !images.is_empty() {
				ollama_msg.x_insert("images", images)?;
			}
			if !tool_calls.is_empty() {
				ollama_msg.x_insert("tool_calls", tool_calls)?;
			}

			if had_tool_responses && leftover_is_empty {
				// The message carried only tool responses (a Tool-role message, or a
				// user message whose embedded responses were all extracted); nothing is
				// left for the carrying message to say, so it is omitted.
			} else {
				messages.push(ollama_msg);
			}

			// Follow-up user message carrying the tool-result images.
			if !tool_response_images.is_empty() {
				messages.push(json!({
					"role": "user",
					"content": TOOL_RESULT_IMAGES_LABEL,
					"images": tool_response_images,
				}));
			}
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

// region:    --- Support

/// Resolve the text content of an Ollama `role:"tool"` message for a `ToolResponse`,
/// pushing its usable image parts (raw base64 only, no data URL) onto
/// `tool_response_images`, which ride in the follow-up `user` image message since
/// Ollama tool messages carry only text content. A response without `parts` keeps its
/// exact legacy text; when parts are present, the `tool_response_fallback_text`
/// placeholder rules apply, tracking only the images contributed by THIS response so
/// the "(see attached image)" placeholder is not emitted for a response whose own
/// parts were all skipped while an earlier response in the same message contributed
/// images. Shared by the Tool-role path and the user-embedded extraction path.
fn tool_response_content_text(tool_response: ToolResponse, tool_response_images: &mut Vec<Arc<str>>) -> String {
	let ToolResponse { content, parts, .. } = tool_response;
	let parts = parts.unwrap_or_default();

	if parts.is_empty() {
		content
	} else {
		let images_count_before = tool_response_images.len();
		for binary in parts {
			if binary.is_image() {
				match binary.source {
					// Note: Ollama native API expects raw base64 (no data URL).
					BinarySource::Base64(data) => tool_response_images.push(data),
					BinarySource::Url(_) => {
						warn!("Ollama native API doesn't support image URLs; skipping tool-result image part");
					}
				}
			} else {
				warn!(
					"ToolResponse binary parts only support images for the Ollama adapter; skipping non-image part '{}'",
					binary.content_type
				);
			}
		}
		let has_own_images = tool_response_images.len() > images_count_before;
		tool_response_fallback_text(content, has_own_images)
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "adapter_shared_tests.rs"]
mod tests;

// endregion: --- Tests
