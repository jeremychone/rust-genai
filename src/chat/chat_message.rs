use crate::chat::{ContentPart, MessageContent, ToolCall, ToolResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};

/// A chat message with a role, multipart content, and optional per-message settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
	/// The message role.
	pub role: ChatRole,

	/// Message content.
	pub content: MessageContent,

	/// Optional per-message options (e.g., cache control).
	pub options: Option<MessageOptions>,
}

// region:    --- Constructors
impl ChatMessage {
	/// Constructs a message for the provided role.
	pub fn new(role: ChatRole, content: impl Into<MessageContent>) -> Self {
		Self {
			role,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a system message.
	pub fn system(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs an assistant message.
	pub fn assistant(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a user message.
	pub fn user(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			options: None,
		}
	}

	/// Constructs a tool message.
	pub fn tool(content: impl Into<MessageContent>) -> Self {
		Self {
			role: ChatRole::Tool,
			content: content.into(),
			options: None,
		}
	}
}
// endregion: --- Constructors

// region:    --- Accessors
impl ChatMessage {
	/// Returns an approximate in-memory size of this `ChatMessage`, in bytes,
	/// computed as the size of the content plus.
	pub fn size(&self) -> usize {
		// Note: Do not include the role len
		self.content.size()
	}
}
// endregion: --- Accessors

// region:    --- Builders
impl ChatMessage {
	/// Attaches options to this message.
	pub fn with_options(mut self, options: impl Into<MessageOptions>) -> Self {
		self.options = Some(options.into());
		self
	}

	/// Attach reasoning content to this message as a `ContentPart::ReasoningContent` part.
	/// This supports round-tripping assistant reasoning between requests.
	pub fn with_reasoning_content(mut self, reasoning: Option<String>) -> Self {
		if let Some(reasoning) = reasoning {
			self.content.push(ContentPart::ReasoningContent(reasoning));
		}
		self
	}

	/// Builds an assistant message with thought signatures ordered before tool calls.
	pub fn assistant_tool_calls_with_thoughts(tool_calls: Vec<ToolCall>, thought_signatures: Vec<String>) -> Self {
		let mut parts: Vec<ContentPart> = thought_signatures.into_iter().map(ContentPart::ThoughtSignature).collect();
		parts.extend(tool_calls.into_iter().map(ContentPart::ToolCall));
		ChatMessage::assistant(MessageContent::from_parts(parts))
	}
}
// endregion: --- Builders

// region:    --- MessageOptions

#[derive(Debug, Clone, Default, Serialize, Deserialize, From)]
/// Per-message options (e.g., cache control).
pub struct MessageOptions {
	#[from]
	/// Per-provider cache behavior hint.
	pub cache_control: Option<CacheControl>,
}

impl MessageOptions {
	/// Sets the per-message cache policy.
	pub fn with_cache_control(mut self, cache_control: impl Into<CacheControl>) -> Self {
		self.cache_control = Some(cache_control.into());
		self
	}
}

/// Provider-neutral prompt cache policy.
///
/// Provider support and request or message-level mapping vary. When mixing ephemeral
/// TTLs, longer-lived entries must precede shorter-lived entries in request order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CacheControl {
	/// Default ephemeral cache (5 minutes TTL).
	Ephemeral,
	/// Memory cache.
	///
	/// Providers without a distinct memory mode may map this to their default cache mode.
	Memory,
	/// Explicit 5-minute TTL cache.
	Ephemeral5m,
	/// Extended 1-hour TTL cache.
	///
	/// **Important:** In some providers, when mixing TTLs, 1-hour cache entries must appear before
	/// any 5-minute cache entries in the request.
	///
	/// Note: Costs 2x base input token price vs 1.25x for 5m.
	Ephemeral1h,
	/// Extended 24-hour TTL cache.
	///
	/// Adapters may clamp this to the longest TTL supported by the provider.
	Ephemeral24h,
}

impl From<CacheControl> for MessageOptions {
	fn from(cache_control: CacheControl) -> Self {
		Self {
			cache_control: Some(cache_control),
		}
	}
}
// endregion: --- MessageOptions

// region:    --- ChatRole

/// Chat roles recognized across providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, derive_more::Display)]
#[allow(missing_docs)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

// endregion: --- ChatRole

// region:    --- Froms

/// Creates an assistant message containing the provided tool calls.
///
/// Thought signatures remain attached to their respective calls. Use
/// [`ChatMessage::assistant_tool_calls_with_thoughts`] when the signatures are
/// separate turn-level content that should precede the calls.
impl From<Vec<ToolCall>> for ChatMessage {
	fn from(tool_calls: Vec<ToolCall>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: MessageContent::from(tool_calls),
			options: None,
		}
	}
}

impl From<ToolResponse> for ChatMessage {
	fn from(value: ToolResponse) -> Self {
		ChatMessage::tool(value)
	}
}

impl From<Vec<ToolResponse>> for ChatMessage {
	fn from(responses: Vec<ToolResponse>) -> Self {
		ChatMessage::tool(MessageContent::from(responses))
	}
}

// endregion: --- Froms

#[cfg(test)]
mod tests {
	use super::*;
	use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType};
	use crate::chat::{ChatOptionsSet, ChatRequest};
	use crate::resolver::AuthData;
	use crate::{ModelIden, ServiceTarget};
	use serde_json::json;

	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	fn signed_tool_calls() -> Vec<ToolCall> {
		vec![
			ToolCall {
				call_id: "call-1".to_string(),
				fn_name: "first".to_string(),
				fn_arguments: json!({}),
				thought_signatures: Some(vec!["sig-1a".to_string(), "sig-1b".to_string()]),
			},
			ToolCall {
				call_id: "call-2".to_string(),
				fn_name: "second".to_string(),
				fn_arguments: json!({}),
				thought_signatures: Some(vec!["sig-2".to_string()]),
			},
		]
	}

	#[test]
	fn from_tool_calls_keeps_signatures_on_their_calls() {
		let message = ChatMessage::from(signed_tool_calls());
		let parts = message.content.parts();

		assert_eq!(parts.len(), 2);
		assert!(matches!(
			&parts[0],
			ContentPart::ToolCall(call)
				if call.thought_signatures.as_deref()
					== Some(["sig-1a".to_string(), "sig-1b".to_string()].as_slice())
		));
		assert!(matches!(
			&parts[1],
			ContentPart::ToolCall(call)
				if call.thought_signatures.as_deref() == Some(["sig-2".to_string()].as_slice())
		));
	}

	#[test]
	fn from_tool_calls_does_not_duplicate_openai_reasoning_items() -> Result<()> {
		let adapter_kind = AdapterKind::OpenAIResp;
		let target = ServiceTarget {
			model: ModelIden::new(adapter_kind, "gpt-5.6"),
			auth: AuthData::from_single("test-key"),
			endpoint: AdapterDispatcher::default_endpoint(adapter_kind),
		};
		let request = ChatRequest::new(vec![ChatMessage::user("go"), ChatMessage::from(signed_tool_calls())]);

		let web_request =
			AdapterDispatcher::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())?;
		let input = web_request.payload["input"].as_array().ok_or("input should be an array")?;
		let signatures = input
			.iter()
			.filter(|item| item["type"] == "reasoning")
			.filter_map(|item| item["encrypted_content"].as_str())
			.collect::<Vec<_>>();

		assert_eq!(signatures, ["sig-1a", "sig-1b", "sig-2"]);
		Ok(())
	}

	#[test]
	fn from_tool_calls_keeps_gemini_signatures_with_their_calls() -> Result<()> {
		let adapter_kind = AdapterKind::Gemini;
		let target = ServiceTarget {
			model: ModelIden::new(adapter_kind, "gemini-3-pro"),
			auth: AuthData::from_single("test-key"),
			endpoint: AdapterDispatcher::default_endpoint(adapter_kind),
		};
		let request = ChatRequest::new(vec![ChatMessage::user("go"), ChatMessage::from(signed_tool_calls())]);

		let web_request =
			AdapterDispatcher::to_web_request_data(target, ServiceType::Chat, request, ChatOptionsSet::default())?;
		let contents = web_request.payload["contents"]
			.as_array()
			.ok_or("contents should be an array")?;
		let model_turn = contents
			.iter()
			.find(|content| content["role"] == "model")
			.ok_or("model turn should be present")?;
		let parts = model_turn["parts"].as_array().ok_or("parts should be an array")?;
		let rendered = parts
			.iter()
			.map(|part| {
				(
					part["functionCall"]["name"].as_str().unwrap_or("-"),
					part["thoughtSignature"].as_str().unwrap_or("-"),
				)
			})
			.collect::<Vec<_>>();

		assert_eq!(rendered, [("first", "sig-1a"), ("second", "sig-2")]);
		Ok(())
	}
}
