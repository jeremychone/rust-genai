//! ChatOptions configures a chat request.
//! - It can be passed to `client::exec_chat(...)`, or
//! - set as a default on the client via `client_config.with_chat_options(...)`.
//!
//! Note 1: Additional client-level defaults may be added over time.
//! Note 2: Kept separate from `ChatRequest` for easier reuse and composition.

use crate::Headers;
use crate::chat::chat_req_response_format::ChatResponseFormat;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Options considered by all `Client::exec_*` chat calls.
///
/// A default can be set on the `Client` during builder configuration.
/// Per-call options take precedence over client defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
	/// Sampling temperature (if supported by the provider).
	pub temperature: Option<f64>,

	/// Maximum tokens to generate (if supported).
	pub max_tokens: Option<u32>,

	/// Nucleus sampling (top-p), if supported.
	pub top_p: Option<f64>,

	/// Sequences that halt generation when encountered.
	pub stop_sequences: Vec<String>,

	// -- Stream Options
	/// (streaming) Capture usage metadata; available in `StreamEnd.captured_usage`.
	pub capture_usage: Option<bool>,

	/// (streaming) Concatenate content chunks; available in `StreamEnd.captured_content`.
	pub capture_content: Option<bool>,

	/// (streaming) Concatenate reasoning chunks; available in `StreamEnd.captured_reasoning_content`.
	pub capture_reasoning_content: Option<bool>,

	/// (streaming) Collect tool calls; available in `StreamEnd.captured_tool_calls`.
	pub capture_tool_calls: Option<bool>,

	/// Capture the raw HTTP body (primarily for debugging/inspection).
	pub capture_raw_body: Option<bool>,

	/// Desired response format (e.g., `ChatResponseFormat::JsonMode` for OpenAI-style JSON mode).
	///
	/// Note: Additional formats may be added in the future.
	pub response_format: Option<ChatResponseFormat>,

	// -- Reasoning options
	/// Extract `` into `ChatResponse.reasoning_content` if present.
	pub normalize_reasoning_content: Option<bool>,

	/// Preferred reasoning effort, when supported by the provider.
	pub reasoning_effort: Option<ReasoningEffort>,

	/// Seed for repeatability, if supported.
	pub seed: Option<u64>,

	/// Additional HTTP headers to include with the request.
	pub extra_headers: Option<Headers>,
}

/// Chainable Setters
impl ChatOptions {
	/// Sets the sampling temperature.
	pub fn with_temperature(mut self, value: f64) -> Self {
		self.temperature = Some(value);
		self
	}

	/// Sets the maximum number of output tokens.
	pub fn with_max_tokens(mut self, value: u32) -> Self {
		self.max_tokens = Some(value);
		self
	}

	/// Sets nucleus sampling (top-p).
	pub fn with_top_p(mut self, value: f64) -> Self {
		self.top_p = Some(value);
		self
	}

	/// Enables or disables capturing usage in streaming mode.
	pub fn with_capture_usage(mut self, value: bool) -> Self {
		self.capture_usage = Some(value);
		self
	}

	/// Enables or disables capturing concatenated content in streaming mode.
	pub fn with_capture_content(mut self, value: bool) -> Self {
		self.capture_content = Some(value);
		self
	}

	/// Enables or disables capturing concatenated reasoning content in streaming mode.
	pub fn with_capture_reasoning_content(mut self, value: bool) -> Self {
		self.capture_reasoning_content = Some(value);
		self
	}

	/// Enables or disables capturing tool calls in streaming mode.
	pub fn with_capture_tool_calls(mut self, value: bool) -> Self {
		self.capture_tool_calls = Some(value);
		self
	}

	/// Enables or disables capturing the raw HTTP body.
	pub fn with_capture_raw_body(mut self, value: bool) -> Self {
		self.capture_raw_body = Some(value);
		self
	}

	/// Sets the stop sequences.
	pub fn with_stop_sequences(mut self, values: Vec<String>) -> Self {
		self.stop_sequences = values;
		self
	}

	/// Enables or disables normalization of reasoning content (e.g., `<think>...</think>`).
	pub fn with_normalize_reasoning_content(mut self, value: bool) -> Self {
		self.normalize_reasoning_content = Some(value);
		self
	}

	/// Sets the response format.
	pub fn with_response_format(mut self, res_format: impl Into<ChatResponseFormat>) -> Self {
		self.response_format = Some(res_format.into());
		self
	}

	/// Sets the reasoning effort hint.
	pub fn with_reasoning_effort(mut self, value: ReasoningEffort) -> Self {
		self.reasoning_effort = Some(value);
		self
	}

	/// Sets the deterministic seed.
	pub fn with_seed(mut self, value: u64) -> Self {
		self.seed = Some(value);
		self
	}

	/// Adds extra HTTP headers.
	pub fn with_extra_headers(mut self, headers: impl Into<Headers>) -> Self {
		self.extra_headers = Some(headers.into());
		self
	}

	// -- Deprecated

	/// Deprecated: use `with_response_format(ChatResponseFormat::JsonMode)`.
	///
	/// When using JSON mode, you should still instruct the model to produce JSON in your prompt
	/// for broad provider compatibility (e.g., mention "json" in system/user messages).
	#[deprecated(note = "Use with_response_format(ChatResponseFormat::JsonMode)")]
	pub fn with_json_mode(mut self, value: bool) -> Self {
		if value {
			self.response_format = Some(ChatResponseFormat::JsonMode);
		}
		self
	}
}

// region:    --- ReasoningEffort

/// Provider-specific hint for reasoning intensity/budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReasoningEffort {
	Minimal,
	Low,
	Medium,
	High,
	Budget(u32),
}

impl ReasoningEffort {
	/// Returns the lowercase variant name; `Budget(_)` returns `"budget"`.
	pub fn variant_name(&self) -> &'static str {
		match self {
			ReasoningEffort::Minimal => "minimal",
			ReasoningEffort::Low => "low",
			ReasoningEffort::Medium => "medium",
			ReasoningEffort::High => "high",
			ReasoningEffort::Budget(_) => "budget",
		}
	}

	/// Returns a keyword for non-`Budget` variants; `None` for `Budget(_)`.
	pub fn as_keyword(&self) -> Option<&'static str> {
		match self {
			ReasoningEffort::Minimal => Some("minimal"),
			ReasoningEffort::Low => Some("low"),
			ReasoningEffort::Medium => Some("medium"),
			ReasoningEffort::High => Some("high"),
			ReasoningEffort::Budget(_) => None,
		}
	}

	/// Parses a keyword into a non-`Budget` variant.
	pub fn from_keyword(name: &str) -> Option<Self> {
		match name {
			"minimal" => Some(ReasoningEffort::Low),
			"low" => Some(ReasoningEffort::Low),
			"medium" => Some(ReasoningEffort::Medium),
			"high" => Some(ReasoningEffort::High),
			_ => None,
		}
	}

	/// If `model_name` ends with `-<effort>`, returns the parsed effort and the trimmed name.
	///
	/// Only keyword variants are produced; `Budget` is never created here.
	/// Returns `(effort, trimmed_model_name)`.
	pub fn from_model_name(model_name: &str) -> (Option<Self>, &str) {
		if let Some((prefix, last)) = model_name.rsplit_once('-')
			&& let Some(effort) = ReasoningEffort::from_keyword(last)
		{
			return (Some(effort), prefix);
		}
		(None, model_name)
	}
}

impl std::fmt::Display for ReasoningEffort {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ReasoningEffort::Minimal => write!(f, "minimal"),
			ReasoningEffort::Low => write!(f, "low"),
			ReasoningEffort::Medium => write!(f, "medium"),
			ReasoningEffort::High => write!(f, "high"),
			ReasoningEffort::Budget(n) => write!(f, "{n}"),
		}
	}
}

impl std::str::FromStr for ReasoningEffort {
	type Err = Error;

	/// Parses a keyword effort or a numeric budget.
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

	pub fn capture_raw_body(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_raw_body)
			.or_else(|| self.client.and_then(|client| client.capture_raw_body))
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

	pub fn seed(&self) -> Option<u64> {
		self.chat
			.and_then(|chat| chat.seed)
			.or_else(|| self.client.and_then(|client| client.seed))
	}

	pub fn extra_headers(&self) -> Option<&Headers> {
		self.chat
			.and_then(|chat| chat.extra_headers.as_ref())
			.or_else(|| self.client.and_then(|client| client.extra_headers.as_ref()))
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

