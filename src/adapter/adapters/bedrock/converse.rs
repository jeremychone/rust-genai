//! genai ChatRequest ↔ Bedrock Converse JSON mapping.
//!
//! Converse normalizes message shape across all Bedrock publishers, so the main wire-format
//! work lives here. Publisher-specific bits (reasoning budget, etc.) go under
//! `additionalModelRequestFields` via [`BedrockPublisher`].

use crate::chat::{
	Binary, BinarySource, ChatOptionsSet, ChatRequest, ChatResponse, ChatRole, ContentPart, MessageContent,
	ReasoningEffort, StopReason, Tool, ToolCall, ToolName, Usage,
};
use crate::webc::WebResponse;
use crate::{Error, ModelIden, Result};
use serde_json::{Map, Value, json};
use tracing::warn;
use value_ext::JsonValueExt;

/// Which Bedrock publisher a model ID targets. Used only to fill
/// `additionalModelRequestFields` — message shape is identical across publishers.
#[derive(Debug, Clone, Copy)]
pub(super) enum BedrockPublisher {
	Anthropic,
	AmazonNova,
	Other,
}

impl BedrockPublisher {
	/// Model IDs are of the form `<publisher>.<model>...` or
	/// `<region>.<publisher>.<model>...` (cross-region inference profiles).
	pub(super) fn from_model_id(model_id: &str) -> Self {
		// Strip an optional leading "us."/"eu."/"apac." inference-profile prefix so we can
		// match the real publisher segment.
		let tail = model_id.split_once('.').map(|(_, rest)| rest).unwrap_or(model_id);
		let publisher_segment = tail.split_once('.').map(|(p, _)| p).unwrap_or(tail);

		// For non-profile IDs, the leading segment IS the publisher.
		let publisher = if publisher_segment.is_empty() {
			model_id.split_once('.').map(|(p, _)| p).unwrap_or(model_id)
		} else {
			publisher_segment
		};

		match publisher {
			"anthropic" => Self::Anthropic,
			"amazon" => Self::AmazonNova, // Nova models; Titan would also hit this
			_ => Self::Other,
		}
	}
}

/// Build the JSON body for a Converse / ConverseStream call.
pub(super) fn build_converse_payload(
	model_iden: &ModelIden,
	chat_req: ChatRequest,
	options_set: ChatOptionsSet<'_, '_>,
) -> Result<Value> {
	let (_, model_name) = model_iden.model_name.namespace_and_name();
	let publisher = BedrockPublisher::from_model_id(model_name);

	let ConverseRequestParts { system, messages, tools } = into_converse_request_parts(chat_req)?;

	let mut payload = json!({ "messages": messages });

	if let Some(system) = system {
		payload.x_insert("system", system)?;
	}

	if let Some(tools) = tools {
		payload.x_insert("toolConfig", json!({ "tools": tools }))?;
	}

	// inferenceConfig
	let mut inference: Map<String, Value> = Map::new();
	let max_tokens = resolve_max_tokens(model_name, &options_set);
	inference.insert("maxTokens".to_string(), json!(max_tokens));
	if let Some(temperature) = options_set.temperature() {
		inference.insert("temperature".to_string(), json!(temperature));
	}
	if let Some(top_p) = options_set.top_p() {
		inference.insert("topP".to_string(), json!(top_p));
	}
	if !options_set.stop_sequences().is_empty() {
		inference.insert("stopSequences".to_string(), json!(options_set.stop_sequences()));
	}
	payload.x_insert("inferenceConfig", Value::Object(inference))?;

	// additionalModelRequestFields — publisher-specific (reasoning, etc.)
	if let Some(effort) = options_set.reasoning_effort() {
		if let Some(additional) = publisher_additional_fields(publisher, effort) {
			payload.x_insert("additionalModelRequestFields", additional)?;
		}
	}

	Ok(payload)
}

/// Parse a Converse JSON response into a genai `ChatResponse`.
pub(super) fn parse_converse_response(
	model_iden: ModelIden,
	web_response: WebResponse,
) -> Result<ChatResponse> {
	let WebResponse { mut body, .. } = web_response;

	// -- Stop reason
	let stop_reason = body
		.x_take::<Option<String>>("stopReason")
		.ok()
		.flatten()
		.map(|s| StopReason::from(normalize_stop_reason(s.as_str()).to_string()));

	// -- Usage
	let usage_value = body.x_take::<Value>("usage").ok();
	let usage = usage_value.map(parse_usage).unwrap_or_default();

	// -- Content — output.message.content is an array of blocks
	let content_items: Vec<Value> = body.x_take("/output/message/content").unwrap_or_default();

	let mut content: MessageContent = MessageContent::default();
	let mut reasoning_content: Vec<String> = Vec::new();

	for mut item in content_items {
		// Each item has exactly one field indicating block type.
		if let Ok(text) = item.x_take::<String>("text") {
			content.push(ContentPart::from_text(text));
		} else if let Ok(mut tool_use) = item.x_take::<Value>("toolUse") {
			let call_id = tool_use.x_take::<String>("toolUseId")?;
			let fn_name = tool_use.x_take::<String>("name")?;
			let fn_arguments = tool_use.x_take::<Value>("input").unwrap_or_default();
			content.push(ContentPart::ToolCall(ToolCall {
				call_id,
				fn_name,
				fn_arguments,
				thought_signatures: None,
			}));
		} else if let Ok(mut reasoning) = item.x_take::<Value>("reasoningContent") {
			// Converse reasoning block: { reasoningText: { text, signature? } }
			if let Ok(text) = reasoning.x_take::<String>("/reasoningText/text") {
				reasoning_content.push(text);
			}
		} else {
			// Unknown block type — preserve as custom for forward-compat.
			content.push(ContentPart::from_custom(item, Some(model_iden.clone())));
		}
	}

	let reasoning_content = if reasoning_content.is_empty() {
		None
	} else {
		Some(reasoning_content.join("\n"))
	};

	let provider_model_iden = model_iden.clone();

	Ok(ChatResponse {
		content,
		reasoning_content,
		model_iden,
		provider_model_iden,
		stop_reason,
		usage,
		captured_raw_body: None,
		response_id: None,
	})
}

/// Map Converse `stopReason` values onto the set expected by `StopReason::from`.
/// Converse values include: end_turn, tool_use, max_tokens, stop_sequence, guardrail_intervened, content_filtered.
pub(super) fn normalize_stop_reason(converse_reason: &str) -> &str {
	// genai's StopReason::from already handles common names ("end_turn", "tool_use", "max_tokens",
	// "stop_sequence"). Pass through as-is.
	converse_reason
}

fn parse_usage(mut usage_value: Value) -> Usage {
	let input_tokens: i32 = usage_value.x_take("inputTokens").ok().unwrap_or(0);
	let output_tokens: i32 = usage_value.x_take("outputTokens").ok().unwrap_or(0);
	let total_tokens: i32 = usage_value.x_take("totalTokens").ok().unwrap_or(input_tokens + output_tokens);

	// Bedrock reports cache stats under cacheReadInputTokens / cacheWriteInputTokens when supported.
	let cache_read: Option<i32> = usage_value.x_take("cacheReadInputTokens").ok();
	let cache_write: Option<i32> = usage_value.x_take("cacheWriteInputTokens").ok();

	let prompt_tokens_details = if cache_read.is_some() || cache_write.is_some() {
		Some(crate::chat::PromptTokensDetails {
			cache_creation_tokens: cache_write,
			cache_creation_details: None,
			cached_tokens: cache_read,
			audio_tokens: None,
		})
	} else {
		None
	};

	Usage {
		prompt_tokens: Some(input_tokens),
		prompt_tokens_details,
		completion_tokens: Some(output_tokens),
		completion_tokens_details: None,
		total_tokens: Some(total_tokens),
	}
}

fn resolve_max_tokens(model_name: &str, options_set: &ChatOptionsSet) -> u32 {
	options_set.max_tokens().unwrap_or_else(|| {
		// Conservative defaults by publisher; most Bedrock publishers require maxTokens in inferenceConfig.
		match BedrockPublisher::from_model_id(model_name) {
			BedrockPublisher::Anthropic => {
				// Mirror the Anthropic adapter's heuristics for parity.
				if model_name.contains("claude-sonnet")
					|| model_name.contains("claude-haiku")
					|| model_name.contains("claude-opus-4-5")
				{
					crate::adapter::adapters::anthropic::MAX_TOKENS_64K.max(64000)
				} else if model_name.contains("claude-opus-4") {
					32000
				} else if model_name.contains("claude-3-5") {
					8192
				} else {
					4096
				}
			}
			BedrockPublisher::AmazonNova => 5000,
			BedrockPublisher::Other => 4096,
		}
	})
}

fn publisher_additional_fields(publisher: BedrockPublisher, effort: &ReasoningEffort) -> Option<Value> {
	match publisher {
		BedrockPublisher::Anthropic => {
			let budget = match effort {
				ReasoningEffort::None => return None,
				ReasoningEffort::Budget(n) => *n,
				ReasoningEffort::Minimal | ReasoningEffort::Low => 1024,
				ReasoningEffort::Medium => 8000,
				ReasoningEffort::High | ReasoningEffort::XHigh | ReasoningEffort::Max => 24000,
			};
			Some(json!({
				"thinking": {
					"type": "enabled",
					"budget_tokens": budget,
				}
			}))
		}
		BedrockPublisher::AmazonNova => {
			// Nova surfaces reasoning via inferenceConfig.reasoningConfig today; when a user explicitly sets
			// ReasoningEffort, opt in.
			match effort {
				ReasoningEffort::None => None,
				_ => Some(json!({
					"inferenceConfig": { "reasoningConfig": { "type": "enabled" } }
				})),
			}
		}
		BedrockPublisher::Other => None,
	}
}

struct ConverseRequestParts {
	system: Option<Value>,
	messages: Vec<Value>,
	tools: Option<Vec<Value>>,
}

/// Translate a genai `ChatRequest` into Converse's `{system, messages, toolConfig}` shape.
fn into_converse_request_parts(chat_req: ChatRequest) -> Result<ConverseRequestParts> {
	let mut messages: Vec<Value> = Vec::new();
	let mut systems: Vec<String> = Vec::new();

	if let Some(system) = chat_req.system {
		systems.push(system);
	}

	for msg in chat_req.messages {
		match msg.role {
			ChatRole::System => {
				if let Some(text) = msg.content.joined_texts() {
					systems.push(text);
				}
			}
			ChatRole::User => {
				let blocks = user_content_to_converse_blocks(msg.content);
				if !blocks.is_empty() {
					messages.push(json!({ "role": "user", "content": blocks }));
				}
			}
			ChatRole::Assistant => {
				let blocks = assistant_content_to_converse_blocks(msg.content);
				if !blocks.is_empty() {
					messages.push(json!({ "role": "assistant", "content": blocks }));
				}
			}
			ChatRole::Tool => {
				// Tool responses become a user message whose content is tool_result blocks.
				let blocks = tool_content_to_converse_blocks(msg.content);
				if !blocks.is_empty() {
					messages.push(json!({ "role": "user", "content": blocks }));
				}
			}
		}
	}

	let system = if systems.is_empty() {
		None
	} else {
		// Converse expects system as an array of {text} blocks.
		let parts: Vec<Value> = systems.into_iter().map(|s| json!({ "text": s })).collect();
		Some(Value::Array(parts))
	};

	let tools: Option<Vec<Value>> = chat_req
		.tools
		.map(|tools| {
			tools
				.into_iter()
				.map(tool_to_converse_tool)
				.collect::<Result<Vec<Value>>>()
		})
		.transpose()?;

	Ok(ConverseRequestParts { system, messages, tools })
}

fn user_content_to_converse_blocks(content: MessageContent) -> Vec<Value> {
	let mut blocks = Vec::new();
	for part in content {
		match part {
			ContentPart::Text(text) => blocks.push(json!({ "text": text })),
			ContentPart::Binary(binary) => {
				if let Some(block) = binary_to_converse_block(binary) {
					blocks.push(block);
				}
			}
			ContentPart::ToolResponse(tool_response) => {
				blocks.push(json!({
					"toolResult": {
						"toolUseId": tool_response.call_id,
						"content": [{ "text": tool_response.content }],
					}
				}));
			}
			// Not valid in user role for Converse — skip.
			ContentPart::ToolCall(_) => {}
			ContentPart::ThoughtSignature(_) => {}
			ContentPart::ReasoningContent(_) => {}
			ContentPart::Custom(_) => {}
		}
	}
	blocks
}

fn assistant_content_to_converse_blocks(content: MessageContent) -> Vec<Value> {
	let mut blocks = Vec::new();
	for part in content {
		match part {
			ContentPart::Text(text) => blocks.push(json!({ "text": text })),
			ContentPart::ToolCall(tool_call) => {
				let input = if tool_call.fn_arguments.is_null() {
					Value::Object(Map::new())
				} else {
					tool_call.fn_arguments
				};
				blocks.push(json!({
					"toolUse": {
						"toolUseId": tool_call.call_id,
						"name": tool_call.fn_name,
						"input": input,
					}
				}));
			}
			// Unsupported in assistant role for Converse.
			ContentPart::Binary(_) => {}
			ContentPart::ToolResponse(_) => {}
			ContentPart::ThoughtSignature(_) => {}
			ContentPart::ReasoningContent(_) => {}
			ContentPart::Custom(_) => {}
		}
	}
	blocks
}

fn tool_content_to_converse_blocks(content: MessageContent) -> Vec<Value> {
	let mut blocks = Vec::new();
	for part in content {
		if let ContentPart::ToolResponse(tool_response) = part {
			blocks.push(json!({
				"toolResult": {
					"toolUseId": tool_response.call_id,
					"content": [{ "text": tool_response.content }],
				}
			}));
		}
	}
	blocks
}

fn binary_to_converse_block(binary: Binary) -> Option<Value> {
	let is_image = binary.is_image();
	let Binary { content_type, source, .. } = binary;

	// Converse format: image blocks use { image: { format, source: { bytes } } }
	// and document blocks use { document: { format, name, source: { bytes } } }.
	// URL-based sources aren't supported here yet.
	let data = match source {
		BinarySource::Base64(data) => data,
		BinarySource::Url(_) => {
			warn!("Bedrock Converse: URL-based binary sources are not yet supported, skipping");
			return None;
		}
	};

	let format = converse_format_from_content_type(&content_type, is_image)?;

	if is_image {
		Some(json!({
			"image": {
				"format": format,
				"source": { "bytes": data },
			}
		}))
	} else {
		Some(json!({
			"document": {
				"format": format,
				"name": "document",
				"source": { "bytes": data },
			}
		}))
	}
}

fn converse_format_from_content_type(content_type: &str, is_image: bool) -> Option<&'static str> {
	if is_image {
		match content_type {
			"image/jpeg" | "image/jpg" => Some("jpeg"),
			"image/png" => Some("png"),
			"image/gif" => Some("gif"),
			"image/webp" => Some("webp"),
			_ => {
				warn!("Bedrock Converse: unsupported image content-type: {content_type}");
				None
			}
		}
	} else {
		match content_type {
			"application/pdf" => Some("pdf"),
			"text/csv" => Some("csv"),
			"application/msword" => Some("doc"),
			"application/vnd.openxmlformats-officedocument.wordprocessingml.document" => Some("docx"),
			"application/vnd.ms-excel" => Some("xls"),
			"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => Some("xlsx"),
			"text/html" => Some("html"),
			"text/plain" => Some("txt"),
			"text/markdown" => Some("md"),
			_ => {
				warn!("Bedrock Converse: unsupported document content-type: {content_type}");
				None
			}
		}
	}
}

fn tool_to_converse_tool(tool: Tool) -> Result<Value> {
	let Tool {
		name,
		description,
		schema,
		config: _,
		..
	} = tool;

	let name = match name {
		ToolName::Custom(name) => name,
		ToolName::WebSearch => {
			return Err(Error::AdapterNotSupported {
				adapter_kind: crate::adapter::AdapterKind::BedrockApi,
				feature: "web_search builtin tool".to_string(),
			});
		}
	};

	let mut tool_spec = json!({
		"name": name,
		"inputSchema": { "json": schema },
	});
	if let Some(description) = description {
		tool_spec.x_insert("description", description)?;
	}

	Ok(json!({ "toolSpec": tool_spec }))
}
