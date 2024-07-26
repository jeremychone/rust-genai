//! ChatOptions allows to customize a chat request.
//! - It can be given at the `client::exec_chat(..)` level as argument,
//! - or set in the client config `client_config.with_chat_options(..)` to be taken as default by all requests
//!
//! Note 1: Later, we will probably allow to set the client
//! Note 2: Splitting it out of the `ChatRequest` object allows for better reusability of each component.
#[derive(Debug, Clone, Default)]
pub struct ChatOptions {
	/// Will be set for this request if Adapter/providers supports it.
	pub temperature: Option<f64>,

	/// Will be set of this request if Adaptper/provider supports it.
	pub max_tokens: Option<u32>,

	/// Will be set of this request if Adaptper/provider supports it.
	pub top_p: Option<f64>,

	/// (for steam only) Capture the meta usage when in stream mode
	/// `StreamEnd` event payload will contain `captured_usage`
	/// > Note: Will capture the `MetaUsage`
	pub capture_usage: Option<bool>,

	/// (for stream only) Capture/concatenate the full message content from all content chunk
	/// `StreamEnd` from `StreamEvent::End(StreamEnd)` will contain `StreamEnd.captured_content`
	pub capture_content: Option<bool>,

	/// Enable JSON mode for supported models
	pub json_mode: Option<bool>,
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
	pub fn with_json_mode(mut self, value: bool) -> Self {
		self.json_mode = Some(value);
		self
	}
}

// region:    --- ChatOptionsSet

/// This is an internal crate struct to resolve the ChatOptions value in a cascading manner.
/// First, try to get the value at the chat level. (ChatOptions from the exec_chat...(...) argument)
/// If value for the property not found, look at the client default one.
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

	pub fn json_mode(&self) -> Option<bool> {
		self.chat
			.and_then(|chat| chat.json_mode)
			.or_else(|| self.client.and_then(|client| client.json_mode))
	}
}

// endregion: --- ChatOptionsSet
