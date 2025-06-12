//! ChatOptions allows customization of a chat request.
//! - It can be provided at the `client::exec_chat(..)` level as an argument,
//! - or set in the client config `client_config.with_chat_options(..)` to be used as the default for all requests
//!
//! Note 1: In the future, we will probably allow setting the client
//! Note 2: Extracting it from the `ChatRequest` object allows for better reusability of each component.

use crate::chat::chat_req_response_format::ChatResponseFormat;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Chat Options that are considered for any `Client::exec...` calls.
///
/// A fallback `ChatOptions` can also be set at the `Client` during the client builder phase
/// ``
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
	/// Will be used for this request if the Adapter/provider supports it.
	pub temperature: Option<f64>,

	/// Will be used for this request if the Adapter/provider supports it.
	pub max_tokens: Option<u32>,

	/// Will be used for this request if the Adapter/provider supports it.
	pub top_p: Option<f64>,

	/// Specifies sequences used as end markers when generating text
	pub stop_sequences: Vec<String>,

	// -- Stream Options
	/// (for streaming only) Capture the meta usage when in stream mode
	/// `StreamEnd` event payload will contain `captured_usage`
	/// > Note: Will capture the `MetaUsage`
	pub capture_usage: Option<bool>,

	/// (for streaming only) Capture/concatenate the full message content from all content chunks
	/// `StreamEnd` from `StreamEvent::End(StreamEnd)` will contain `StreamEnd.captured_content`
	pub capture_content: Option<bool>,

	/// (for streaming only) Capture/concatenate the full message content from all content chunks
	/// `StreamEnd` from `StreamEvent::End(StreamEnd)` will contain `StreamEnd.captured_reasoning_content`
	pub capture_reasoning_content: Option<bool>,

	/// (for streaming only) Capture all tool calls from tool call chunks
	/// `StreamEnd` from `StreamEvent::End(StreamEnd)` will contain `StreamEnd.captured_tool_calls`
	pub capture_tool_calls: Option<bool>,

	/// Specifies the response format for a chat request.
	/// - `ChatResponseFormat::JsonMode` is for OpenAI-like API usage, where the user must specify in the prompt that they want a JSON format response.
	///
	/// NOTE: More response formats are coming soon.
	pub response_format: Option<ChatResponseFormat>,

	// -- Reasoning options
	/// Denote if the content should be parsed to extract eventual `<think>...</think>` content
	/// into `ChatResponse.reasoning_content`
	pub normalize_reasoning_content: Option<bool>,

	pub reasoning_effort: Option<ReasoningEffort>,
}

/// Chainable Setters
impl ChatOptions {
	/// Set the `temperature` for this request.
	pub fn with_temperature(mut self, value: f64) -> Self {
		self.temperature = Some(value);
		self
	}

	/// Set the `max_tokens` for this request.
	pub fn with_max_tokens(mut self, value: u32) -> Self {
		self.max_tokens = Some(value);
		self
	}

	/// Set the `top_p` for this request.
	pub fn with_top_p(mut self, value: f64) -> Self {
		self.top_p = Some(value);
		self
	}

	/// Set the `capture_usage` for this request.
	pub fn with_capture_usage(mut self, value: bool) -> Self {
		self.capture_usage = Some(value);
		self
	}

	/// Set the `capture_content` for this request.
	pub fn with_capture_content(mut self, value: bool) -> Self {
		self.capture_content = Some(value);
		self
	}

	/// Set the `capture_reasoning_content` for this request.
	pub fn with_capture_reasoning_content(mut self, value: bool) -> Self {
		self.capture_reasoning_content = Some(value);
		self
	}

	/// Set the `capture_tool_calls` for this request.
	pub fn with_capture_tool_calls(mut self, value: bool) -> Self {
		self.capture_tool_calls = Some(value);
		self
	}

	pub fn with_stop_sequences(mut self, values: Vec<String>) -> Self {
		self.stop_sequences = values;
		self
	}

	pub fn with_normalize_reasoning_content(mut self, value: bool) -> Self {
		self.normalize_reasoning_content = Some(value);
		self
	}

	/// Set the `response_format` for this request.
	pub fn with_response_format(mut self, res_format: impl Into<ChatResponseFormat>) -> Self {
		self.response_format = Some(res_format.into());
		self
	}

	pub fn with_reasoning_effort(mut self, value: ReasoningEffort) -> Self {
		self.reasoning_effort = Some(value);
		self
	}

	// -- Deprecated

	/// Set the `json_mode` for this request.
	///
	/// IMPORTANT: This is deprecated now; use `with_response_format(ChatResponseFormat::JsonMode)`
	///
	/// IMPORTANT: When this is JsonMode, it's important to instruct the model to produce JSON yourself
	///            for many models/providers to work correctly. This can be approximately done
	///            by checking if any System and potentially User messages contain `"json"`
	///            (make sure to check the `.system` property as well).
	#[deprecated(note = "Use with_response_format(ChatResponseFormat::JsonMode)")]
	pub fn with_json_mode(mut self, value: bool) -> Self {
		if value {
			self.response_format = Some(ChatResponseFormat::JsonMode);
		}
		self
	}
}

// region:    --- ReasoningEffort

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningEffort {
	Low,
	Medium,
	High,
	Budget(u32),
}

impl ReasoningEffort {
	/// Returns the lowercase, static variant name.
	/// Budget always returns "budget" regardless of the number.
	pub fn variant_name(&self) -> &'static str {
		match self {
			ReasoningEffort::Low => "low",
			ReasoningEffort::Medium => "medium",
			ReasoningEffort::High => "high",
			ReasoningEffort::Budget(_) => "budget",
		}
	}

	/// Keywords are just the "high", "medium", "low",
	/// Budget will be None
	pub fn as_keyword(&self) -> Option<&'static str> {
		match self {
			ReasoningEffort::Low => Some("low"),
			ReasoningEffort::Medium => Some("medium"),
			ReasoningEffort::High => Some("high"),
			ReasoningEffort::Budget(_) => None,
		}
	}

	/// Keywords are just the "high", "medium", "low",
	/// This function will not create budget variant (no)
	pub fn from_keyword(name: &str) -> Option<Self> {
		match name {
			"low" => Some(ReasoningEffort::Low),
			"medium" => Some(ReasoningEffort::Medium),
			"high" => Some(ReasoningEffort::High),
			_ => None,
		}
	}

	/// If the model_name ends with the lowercase string of a ReasoningEffort variant,
	/// return the ReasoningEffort and the trimmed model_name.
	///
	/// Otherwise, return the model_name as is.
	///
	/// This will not create budget variant, only the keyword one
	/// Returns (reasoning_effort, model_name)
	pub fn from_model_name(model_name: &str) -> (Option<Self>, &str) {
		if let Some((prefix, last)) = model_name.rsplit_once('-') {
			if let Some(effort) = ReasoningEffort::from_keyword(last) {
				return (Some(effort), prefix);
			}
		}
		(None, model_name)
	}
}

impl std::fmt::Display for ReasoningEffort {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ReasoningEffort::Low => write!(f, "low"),
			ReasoningEffort::Medium => write!(f, "medium"),
			ReasoningEffort::High => write!(f, "high"),
			ReasoningEffort::Budget(n) => write!(f, "{}", n),
		}
	}
}

impl std::str::FromStr for ReasoningEffort {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self> {
		Self::from_keyword(s)
			.or_else(|| s.parse::<u32>().ok().map(Self::Budget))
			.ok_or(Error::ReasoningParsingError { actual: s.to_string() })
	}
}

// endregion: --- ReasoningEffort

// region:    --- ChatOptionsSet

/// This is an internal crate struct to resolve the ChatOptions value in a cascading manner.
/// First, it attempts to get the value at the chat level (ChatOptions from the exec_chat...(...) argument).
/// If a value for the property is not found, it looks at the client default one.
#[derive(Default, Clone)]
pub(crate) struct ChatOptionsSet<'a, 'b> {
	client: Option<&'a ChatOptions>,
	chat: Option<&'b ChatOptions>,
}

impl<'a, 'b> ChatOptionsSet<'a, 'b> {
	pub fn with_client_options(mut self, options: Option<&'a ChatOptions>) -> Self {
		self.client = options;
		self
	}
	pub fn with_chat_options(mut self, options: Option<&'b ChatOptions>) -> Self {
		self.chat = options;
		self
	}
}

impl ChatOptionsSet<'_, '_> {
	pub fn temperature(&self) -> Option<f64> {
		self.chat
			.and_then(|chat| chat.temperature)
			.or_else(|| self.client.and_then(|client| client.temperature))
	}

	pub fn max_tokens(&self) -> Option<u32> {
		self.chat
			.and_then(|chat| chat.max_tokens)
			.or_else(|| self.client.and_then(|client| client.max_tokens))
	}

	pub fn top_p(&self) -> Option<f64> {
		self.chat
			.and_then(|chat| chat.top_p)
			.or_else(|| self.client.and_then(|client| client.top_p))
	}

	pub fn stop_sequences(&self) -> &[String] {
		self.chat
			.map(|chat| chat.stop_sequences.deref())
			.or_else(|| self.client.map(|client| client.stop_sequences.deref()))
			.unwrap_or(&[])
	}

	pub fn capture_usage(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_usage)
			.or_else(|| self.client.and_then(|client| client.capture_usage))
	}

	pub fn capture_content(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_content)
			.or_else(|| self.client.and_then(|client| client.capture_content))
	}

	pub fn capture_reasoning_content(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_reasoning_content)
			.or_else(|| self.client.and_then(|client| client.capture_reasoning_content))
	}
	
	pub fn capture_tool_calls(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_tool_calls)
			.or_else(|| self.client.and_then(|client| client.capture_tool_calls))
	}

	pub fn response_format(&self) -> Option<&ChatResponseFormat> {
		self.chat
			.and_then(|chat| chat.response_format.as_ref())
			.or_else(|| self.client.and_then(|client| client.response_format.as_ref()))
	}

	pub fn normalize_reasoning_content(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.normalize_reasoning_content)
			.or_else(|| self.client.and_then(|client| client.normalize_reasoning_content))
	}

	pub fn reasoning_effort(&self) -> Option<&ReasoningEffort> {
		self.chat
			.and_then(|chat| chat.reasoning_effort.as_ref())
			.or_else(|| self.client.and_then(|client| client.reasoning_effort.as_ref()))
	}

	/// Returns true only if there is a ChatResponseFormat::JsonMode
	#[deprecated(note = "Use .response_format()")]
	#[allow(unused)]
	pub fn json_mode(&self) -> Option<bool> {
		match self.response_format() {
			Some(ChatResponseFormat::JsonMode) => Some(true),
			None => None,
			_ => Some(false),
		}
	}
}

// endregion: --- ChatOptionsSet
