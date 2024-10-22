//! ChatOptions allows customization of a chat request.
//! - It can be provided at the `client::exec_chat(..)` level as an argument,
//! - or set in the client config `client_config.with_chat_options(..)` to be used as default for all requests
//!
//! Note 1: In the future, we will probably allow setting the client
//! Note 2: Extracting it from the `ChatRequest` object allows for better reusability of each component.

use crate::chat::chat_req_response_format::ChatResponseFormat;
use serde::{Deserialize, Serialize};

/// Chat Options that are taken into account for any `Client::exec...` calls.
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

	/// (for streaming only) Capture the meta usage when in stream mode
	/// `StreamEnd` event payload will contain `captured_usage`
	/// > Note: Will capture the `MetaUsage`
	pub capture_usage: Option<bool>,

	/// (for streaming only) Capture/concatenate the full message content from all content chunks
	/// `StreamEnd` from `StreamEvent::End(StreamEnd)` will contain `StreamEnd.captured_content`
	pub capture_content: Option<bool>,

	/// Specifies the response format for a chat request.
	/// - `ChatResponseFormat::JsonMode` is for OpenAI-like API usage, where the user must specify in the prompt that they want a JSON format response.
	///
	/// NOTE: More response formats are coming soon.
	pub response_format: Option<ChatResponseFormat>,
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

	/// Set the `response_format` for this request.
	pub fn with_response_format(mut self, res_format: impl Into<ChatResponseFormat>) -> Self {
		self.response_format = Some(res_format.into());
		self
	}
}

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

	pub fn capture_usage(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_usage)
			.or_else(|| self.client.and_then(|client| client.capture_usage))
	}

	#[allow(unused)] // for now, until implemented
	pub fn capture_content(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.capture_content)
			.or_else(|| self.client.and_then(|client| client.capture_content))
	}

	pub fn response_format(&self) -> Option<&ChatResponseFormat> {
		self.chat
			.and_then(|chat| chat.response_format.as_ref())
			.or_else(|| self.client.and_then(|client| client.response_format.as_ref()))
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
