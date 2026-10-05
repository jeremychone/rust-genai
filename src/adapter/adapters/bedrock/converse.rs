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
	OpenAI,
	Other,
}

const INFERENCE_PROFILE_PREFIXES: &[&str] = &["us", "us-gov", "eu", "apac", "au", "ca", "jp", "in", "global"];

impl BedrockPublisher {
	/// Model IDs are of the form `<publisher>.<model>...` or
	/// `<geo>.<publisher>.<model>...` (cross-region inference profiles).
	pub(super) fn from_model_id(model_id: &str) -> Self {
		let mut segments = model_id.split('.');
		let first = segments.next().unwrap_or_default();
		let publisher = if INFERENCE_PROFILE_PREFIXES.contains(&first) {
			segments.next().unwrap_or_default()
		} else {
			first
		};

		match publisher {
			"anthropic" => Self::Anthropic,
			"amazon" => Self::AmazonNova, // Nova models; Titan would also hit this
			"openai" => Self::OpenAI,
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

	let ConverseRequestParts {
		system,
		messages,
		tools,
	} = into_converse_request_parts(chat_req)?;

	let mut payload = json!({});

	if let Some(system) = system {
		payload.x_insert("system", system)?;
	}

	// -- Tools (before messages)
	if let Some(tools) = tools {
		payload.x_insert("toolConfig", json!({ "tools": tools }))?;
	}

	// -- Messages (after tools)
	payload.x_insert("messages", messages)?;

	// inferenceConfig
	let mut inference: Map<String, Value> = Map::new();
	if nova_high_effort(publisher, options_set.reasoning_effort()) {
		// Nova rejects maxTokens, temperature and topP alongside high reasoning effort.
		if options_set.max_tokens().is_some() || options_set.temperature().is_some() || options_set.top_p().is_some() {
			warn!(
				"Bedrock Nova: max_tokens, temperature and top_p are not allowed with high reasoning effort; omitting them"
			);
		}
	} else {
		let max_tokens = resolve_max_tokens(model_name, &options_set);
		inference.insert("maxTokens".to_string(), json!(max_tokens));
		if let Some(temperature) = options_set.temperature() {
			inference.insert("temperature".to_string(), json!(temperature));
		}
		if let Some(top_p) = options_set.top_p() {
			inference.insert("topP".to_string(), json!(top_p));
		}
	}
	if !options_set.stop_sequences().is_empty() {
		inference.insert("stopSequences".to_string(), json!(options_set.stop_sequences()));
	}
	payload.x_insert("inferenceConfig", Value::Object(inference))?;

	// additionalModelRequestFields — publisher-specific (reasoning, etc.)
	if let Some(effort) = options_set.reasoning_effort()
		&& let Some(additional) = publisher_additional_fields(publisher, effort)
	{
		payload.x_insert("additionalModelRequestFields", additional)?;
	}

	Ok(payload)
}

/// Parse a Converse JSON response into a genai `ChatResponse`.
pub(super) fn parse_converse_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
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
	let mut thought_signatures: Vec<String> = Vec::new();

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
			// Converse reasoning block: `{ reasoningText: { text, signature? } }` or
			// `{ redactedContent: <base64> }`. Signed and redacted blocks are kept as content
			// parts so the turn can be replayed (see `assistant_content_to_converse_blocks`).
			if let Ok(redacted) = reasoning.x_take::<String>("redactedContent") {
				thought_signatures.push(redacted.clone());
				content.push(ContentPart::ThoughtSignature(redacted));
			} else if let Ok(text) = reasoning.x_take::<String>("/reasoningText/text") {
				if let Ok(signature) = reasoning.x_take::<String>("/reasoningText/signature") {
					thought_signatures.push(signature.clone());
					content.push(ContentPart::ThoughtSignature(signature));
					content.push(ContentPart::ReasoningContent(text.clone()));
				}
				reasoning_content.push(text);
			}
		} else {
			// Unknown block type — preserve as custom for forward-compat.
			content.push(ContentPart::from_custom(item, Some(model_iden.clone())));
		}
	}

	// Mirror the signatures onto the first tool call, as the streaming `StreamEnd` does, for
	// callers that only keep the tool calls.
	if !thought_signatures.is_empty()
		&& let Some(tool_call) = content.iter_mut().find_map(|part| match part {
			ContentPart::ToolCall(tool_call) => Some(tool_call),
			_ => None,
		}) {
		tool_call.thought_signatures = Some(thought_signatures);
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

pub(super) fn parse_usage(mut usage_value: Value) -> Usage {
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
			// Reasoning tokens count against the output budget.
			BedrockPublisher::OpenAI => 16384,
			BedrockPublisher::Other => 4096,
		}
	})
}

fn publisher_additional_fields(publisher: BedrockPublisher, effort: &ReasoningEffort) -> Option<Value> {
	match publisher {
		BedrockPublisher::Anthropic => {
			let budget = match effort {
				ReasoningEffort::Zero => return None,
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
			// Nova 2 takes `reasoningConfig { type, maxReasoningEffort }` (required when enabled).
			let reasoning_config = match nova_reasoning_effort(effort) {
				Some(level) => json!({ "type": "enabled", "maxReasoningEffort": level }),
				None => json!({ "type": "disabled" }),
			};
			Some(json!({ "inferenceConfig": { "reasoningConfig": reasoning_config } }))
		}
		BedrockPublisher::OpenAI => {
			// Passed through to the OpenAI Responses API, which takes `reasoning.effort`.
			// `reasoning.summary` is rejected on Bedrock, so reasoning only comes back encrypted.
			let effort = match effort {
				ReasoningEffort::Zero => "none",
				other => other.as_keyword()?,
			};
			Some(json!({ "reasoning": { "effort": effort } }))
		}
		BedrockPublisher::Other => None,
	}
}

/// Nova's `maxReasoningEffort` for a genai effort, or `None` for reasoning off.
fn nova_reasoning_effort(effort: &ReasoningEffort) -> Option<&'static str> {
	match effort {
		ReasoningEffort::Zero => None,
		ReasoningEffort::Minimal | ReasoningEffort::Low => Some("low"),
		ReasoningEffort::Medium => Some("medium"),
		ReasoningEffort::High | ReasoningEffort::XHigh | ReasoningEffort::Max => Some("high"),
		ReasoningEffort::Budget(n) if *n <= 1024 => Some("low"),
		ReasoningEffort::Budget(n) if *n <= 8000 => Some("medium"),
		ReasoningEffort::Budget(_) => Some("high"),
	}
}

fn nova_high_effort(publisher: BedrockPublisher, effort: Option<&ReasoningEffort>) -> bool {
	matches!(publisher, BedrockPublisher::AmazonNova) && effort.and_then(nova_reasoning_effort) == Some("high")
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
		.map(|tools| tools.into_iter().map(tool_to_converse_tool).collect::<Result<Vec<Value>>>())
		.transpose()?;

	Ok(ConverseRequestParts {
		system,
		messages,
		tools,
	})
}

fn user_content_to_converse_blocks(content: MessageContent) -> Vec<Value> {
	let mut blocks = Vec::new();
	let mut document_names: Vec<String> = Vec::new();
	for part in content {
		match part {
			ContentPart::Text(text) => blocks.push(json!({ "text": text })),
			ContentPart::Binary(binary) => {
				if let Some(block) = binary_to_converse_block(binary, &mut document_names) {
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

/// Assistant turns replay their reasoning blocks ahead of `toolUse`, so the model continues from
/// its own reasoning after a tool result. Parts arrive as `StreamEnd::captured_content` /
/// `parse_converse_response` lay them out: a `ThoughtSignature` immediately followed by its
/// `ReasoningContent` is a signed `reasoningText` block, and a lone `ThoughtSignature` is a
/// `redactedContent` blob. Unsigned reasoning text can't be replayed and is dropped. When the
/// content carries no signatures, the ones mirrored onto the first tool call are used.
fn assistant_content_to_converse_blocks(content: MessageContent) -> Vec<Value> {
	let mut reasoning_blocks: Vec<Value> = Vec::new();
	let mut other_parts: Vec<ContentPart> = Vec::new();
	let mut parts = content.into_iter().peekable();
	while let Some(part) = parts.next() {
		match part {
			ContentPart::ThoughtSignature(signature) => {
				if let Some(ContentPart::ReasoningContent(_)) = parts.peek()
					&& let Some(ContentPart::ReasoningContent(text)) = parts.next()
				{
					reasoning_blocks.push(json!({
						"reasoningContent": { "reasoningText": { "text": text, "signature": signature } }
					}));
				} else {
					reasoning_blocks.push(json!({ "reasoningContent": { "redactedContent": signature } }));
				}
			}
			ContentPart::ReasoningContent(_) => {}
			other => other_parts.push(other),
		}
	}

	if reasoning_blocks.is_empty()
		&& let Some(mirrored) = other_parts.iter().find_map(|part| match part {
			ContentPart::ToolCall(tool_call) => tool_call.thought_signatures.clone(),
			_ => None,
		}) {
		reasoning_blocks.extend(
			mirrored
				.into_iter()
				.map(|blob| json!({ "reasoningContent": { "redactedContent": blob } })),
		);
	}

	let mut blocks = reasoning_blocks;
	for part in other_parts {
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
			ContentPart::Custom(_) => {}
			// Consumed above.
			ContentPart::ThoughtSignature(_) | ContentPart::ReasoningContent(_) => {}
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

fn binary_to_converse_block(binary: Binary, document_names: &mut Vec<String>) -> Option<Value> {
	let is_image = binary.is_image();
	let Binary {
		content_type,
		source,
		name,
	} = binary;

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
		let name = unique_document_name(name.as_deref(), document_names);
		Some(json!({
			"document": {
				"format": format,
				"name": name,
				"source": { "bytes": data },
			}
		}))
	}
}

/// Converse document names must be unique within a message and may only contain alphanumerics,
/// single spaces, hyphens, parentheses and square brackets. Derive one from the file name.
fn unique_document_name(file_name: Option<&str>, used: &mut Vec<String>) -> String {
	let stem = file_name
		.map(|name| name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name))
		.unwrap_or_default();
	let sanitized: String = stem
		.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '(' | ')' | '[' | ']') {
				c
			} else {
				'-'
			}
		})
		.collect();
	let base = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
	let base = if base.is_empty() { "document".to_string() } else { base };

	let mut name = base.clone();
	let mut n = 2;
	while used.contains(&name) {
		name = format!("{base}-{n}");
		n += 1;
	}
	used.push(name.clone());
	name
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

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use crate::adapter::AdapterKind;
	use crate::chat::{ChatMessage, ChatOptions};

	fn payload_for(model: &str, chat_req: ChatRequest, options: &ChatOptions) -> Result<Value> {
		let model_iden = ModelIden::new(AdapterKind::BedrockApi, model);
		let options_set = ChatOptionsSet::default().with_chat_options(Some(options));
		Ok(build_converse_payload(&model_iden, chat_req, options_set)?)
	}

	fn tool_call() -> ToolCall {
		ToolCall {
			call_id: "call_1".to_string(),
			fn_name: "weather".to_string(),
			fn_arguments: json!({}),
			thought_signatures: None,
		}
	}

	#[test]
	fn publisher_is_read_with_and_without_a_profile_prefix() {
		for (id, expected) in [
			("anthropic.claude-sonnet-4-5-20250929-v1:0", "Anthropic"),
			("us.anthropic.claude-sonnet-4-5-20250929-v1:0", "Anthropic"),
			("global.anthropic.claude-haiku-4-5-20251001-v1:0", "Anthropic"),
			("amazon.nova-pro-v1:0", "AmazonNova"),
			("apac.amazon.nova-lite-v1:0", "AmazonNova"),
			("global.openai.gpt-5.6-terra", "OpenAI"),
			("openai.gpt-5.4", "OpenAI"),
			("meta.llama3-1-70b-instruct-v1:0", "Other"),
		] {
			assert_eq!(format!("{:?}", BedrockPublisher::from_model_id(id)), expected, "{id}");
		}
	}

	#[test]
	fn bare_anthropic_id_gets_thinking_config() -> Result<()> {
		// -- Setup & Fixtures
		let options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Low);
		let chat_req = ChatRequest::new(vec![ChatMessage::user("hi")]);

		// -- Exec
		let payload = payload_for("anthropic.claude-sonnet-4-5-20250929-v1:0", chat_req, &options)?;

		// -- Check
		assert_eq!(
			payload["additionalModelRequestFields"]["thinking"]["budget_tokens"],
			1024
		);
		Ok(())
	}

	#[test]
	fn nova_gets_a_reasoning_effort_level() -> Result<()> {
		// -- Setup & Fixtures
		let chat_req = || ChatRequest::new(vec![ChatMessage::user("hi")]);
		let medium = ChatOptions::default()
			.with_reasoning_effort(ReasoningEffort::Medium)
			.with_temperature(0.5);
		let zero = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Zero);

		// -- Exec
		let medium = payload_for("us.amazon.nova-2-lite-v1:0", chat_req(), &medium)?;
		let zero = payload_for("us.amazon.nova-2-lite-v1:0", chat_req(), &zero)?;

		// -- Check
		assert_eq!(
			medium["additionalModelRequestFields"],
			json!({ "inferenceConfig": { "reasoningConfig": { "type": "enabled", "maxReasoningEffort": "medium" } } })
		);
		assert_eq!(
			medium["inferenceConfig"],
			json!({ "maxTokens": 5000, "temperature": 0.5 })
		);
		assert_eq!(
			zero["additionalModelRequestFields"],
			json!({ "inferenceConfig": { "reasoningConfig": { "type": "disabled" } } })
		);
		Ok(())
	}

	#[test]
	fn nova_high_effort_drops_sampling_settings() -> Result<()> {
		// -- Setup & Fixtures
		let options = ChatOptions::default()
			.with_reasoning_effort(ReasoningEffort::High)
			.with_max_tokens(4000)
			.with_temperature(0.5)
			.with_top_p(0.9);
		let chat_req = ChatRequest::new(vec![ChatMessage::user("hi")]);

		// -- Exec
		let payload = payload_for("amazon.nova-2-lite-v1:0", chat_req, &options)?;

		// -- Check
		assert_eq!(payload["inferenceConfig"], json!({}));
		assert_eq!(
			payload["additionalModelRequestFields"]["inferenceConfig"]["reasoningConfig"]["maxReasoningEffort"],
			"high"
		);
		Ok(())
	}

	#[test]
	fn openai_gets_reasoning_effort() -> Result<()> {
		// -- Setup & Fixtures
		let options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Medium);
		let chat_req = ChatRequest::new(vec![ChatMessage::user("hi")]);

		// -- Exec
		let payload = payload_for("global.openai.gpt-5.6-terra", chat_req, &options)?;

		// -- Check
		assert_eq!(
			payload["additionalModelRequestFields"],
			json!({ "reasoning": { "effort": "medium" } })
		);
		Ok(())
	}

	#[test]
	fn assistant_turn_replays_signed_and_redacted_reasoning_before_tool_use() -> Result<()> {
		// -- Setup & Fixtures
		let assistant = ChatMessage::assistant(MessageContent::from_parts(vec![
			ContentPart::ThoughtSignature("sig-1".to_string()),
			ContentPart::ReasoningContent("thinking".to_string()),
			ContentPart::ThoughtSignature("blob".to_string()),
			ContentPart::from_text("Checking."),
			ContentPart::ToolCall(tool_call()),
		]));
		let chat_req = ChatRequest::new(vec![ChatMessage::user("hi"), assistant]);

		// -- Exec
		let payload = payload_for("us.anthropic.claude-haiku-4-5", chat_req, &ChatOptions::default())?;

		// -- Check
		assert_eq!(
			payload["messages"][1]["content"],
			json!([
				{ "reasoningContent": { "reasoningText": { "text": "thinking", "signature": "sig-1" } } },
				{ "reasoningContent": { "redactedContent": "blob" } },
				{ "text": "Checking." },
				{ "toolUse": { "toolUseId": "call_1", "name": "weather", "input": {} } },
			])
		);
		Ok(())
	}

	#[test]
	fn assistant_turn_falls_back_to_tool_call_signatures() -> Result<()> {
		// -- Setup & Fixtures
		let mut call = tool_call();
		call.thought_signatures = Some(vec!["blob".to_string()]);
		let assistant = ChatMessage::assistant(MessageContent::from_parts(vec![ContentPart::ToolCall(call)]));
		let chat_req = ChatRequest::new(vec![ChatMessage::user("hi"), assistant]);

		// -- Exec
		let payload = payload_for("global.openai.gpt-5.6-terra", chat_req, &ChatOptions::default())?;

		// -- Check
		assert_eq!(
			payload["messages"][1]["content"][0],
			json!({ "reasoningContent": { "redactedContent": "blob" } })
		);
		Ok(())
	}

	#[test]
	fn documents_get_unique_valid_names() -> Result<()> {
		// -- Setup & Fixtures
		let user = ChatMessage::user(MessageContent::from_parts(vec![
			ContentPart::from_binary_base64("application/pdf", "AAAA", Some("report.v2.pdf".to_string())),
			ContentPart::from_binary_base64("application/pdf", "BBBB", Some("report.v2.pdf".to_string())),
			ContentPart::from_binary_base64("text/plain", "CCCC", None),
		]));
		let chat_req = ChatRequest::new(vec![user]);

		// -- Exec
		let payload = payload_for("amazon.nova-lite-v1:0", chat_req, &ChatOptions::default())?;

		// -- Check
		let names: Vec<&str> = payload["messages"][0]["content"]
			.as_array()
			.ok_or("content should be an array")?
			.iter()
			.filter_map(|block| block["document"]["name"].as_str())
			.collect();
		assert_eq!(names, ["report-v2", "report-v2-2", "document"]);
		Ok(())
	}

	#[test]
	fn response_keeps_reasoning_blocks_for_replay() -> Result<()> {
		// -- Setup & Fixtures
		let body = json!({
			"output": { "message": { "role": "assistant", "content": [
				{ "reasoningContent": { "reasoningText": { "text": "thinking", "signature": "sig-1" } } },
				{ "reasoningContent": { "redactedContent": "blob" } },
				{ "toolUse": { "toolUseId": "call_1", "name": "weather", "input": {} } },
			]}},
			"stopReason": "tool_use",
			"usage": { "inputTokens": 1, "outputTokens": 1, "totalTokens": 2 },
		});
		let web_response = WebResponse {
			status: reqwest::StatusCode::OK,
			body,
		};

		// -- Exec
		let res = parse_converse_response(ModelIden::new(AdapterKind::BedrockApi, "m"), web_response)?;

		// -- Check
		assert_eq!(res.reasoning_content.as_deref(), Some("thinking"));
		let parts: Vec<String> = res
			.content
			.parts()
			.iter()
			.map(|part| match part {
				ContentPart::ThoughtSignature(sig) => format!("sig:{sig}"),
				ContentPart::ReasoningContent(text) => format!("text:{text}"),
				ContentPart::ToolCall(call) => format!("call:{:?}", call.thought_signatures),
				_ => "other".to_string(),
			})
			.collect();
		assert_eq!(
			parts,
			["sig:sig-1", "text:thinking", "sig:blob", r#"call:Some(["sig-1", "blob"])"#]
		);
		Ok(())
	}
}

// endregion: --- Tests
