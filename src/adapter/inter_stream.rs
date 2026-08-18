//! Internal stream event types that serve as intermediaries between the provider event and the GenAI stream event.
//!
//! This allows for flexibility if we want to capture events across providers that do not need to
//! be reflected in the public ChatStream event.
//!
//! NOTE: This might be removed at some point as it may not be needed, and we could go directly to the GenAI stream.

use crate::chat::{ContentPart, MessageContent, StopReason, ToolCall, Usage};

#[derive(Debug, Default)]
pub struct InterStreamEnd {
	// When `ChatOptions..capture_usage == true`
	pub captured_usage: Option<Usage>,

	// Normalised stop reason.
	pub captured_stop_reason: Option<StopReason>,

	// Exact ordered assistant content captured by the provider streamer.
	pub captured_content: Option<MessageContent>,

	// When `ChatOptions..capture_reasoning_content == true`
	pub captured_reasoning_content: Option<String>,

	// Response ID for stateful sessions (OpenAI Responses API).
	pub captured_response_id: Option<String>,
}

/// Assemble the legacy split capture accumulators into the one ordered terminal
/// content representation. Provider streamers with native block ordering should
/// construct `MessageContent` directly instead.
pub(crate) fn assemble_captured_content(
	text: Option<String>,
	mut tool_calls: Option<Vec<ToolCall>>,
	thought_signatures: Option<Vec<String>>,
) -> Option<MessageContent> {
	let mut parts = Vec::new();
	if let Some(thought_signatures) = thought_signatures {
		if let Some(first_call) = tool_calls.as_mut().and_then(|calls| calls.first_mut()) {
			first_call.thought_signatures = Some(thought_signatures.clone());
		}
		parts.extend(thought_signatures.into_iter().map(ContentPart::ThoughtSignature));
	}
	if let Some(text) = text {
		parts.push(ContentPart::Text(text));
	}
	if let Some(tool_calls) = tool_calls {
		parts.extend(tool_calls.into_iter().map(ContentPart::ToolCall));
	}
	(!parts.is_empty()).then(|| MessageContent::from_parts(parts))
}

/// Intermediary StreamEvent
#[derive(Debug)]
pub enum InterStreamEvent {
	Start,
	Chunk(String),
	ReasoningChunk(String),
	ThoughtSignatureChunk(String),
	ToolCallChunk(crate::chat::ToolCall),
	Heartbeat,
	End(InterStreamEnd),
}
