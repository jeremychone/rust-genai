//! ChatRequestOptions is a struct that can be passed into the `client::exec_chat...` as the last argument
//! to customize the request behavior per call.
//! Note: Splitting it out of the `ChatRequest` object allows for better reusability of each component.
//!
//! IMPORTANT: These are not implemented yet, but here to show some of the directions and start having them part of the client APIs.

#[derive(Debug, Clone, Default)]
pub struct ChatRequestOptions {
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
}

/// Chainable Setters
impl ChatRequestOptions {
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
}

// region:    --- ChatRequestOptionsSet

#[derive(Default, Clone)]
pub(crate) struct ChatRequestOptionsSet<'a, 'b> {
	client: Option<&'a ChatRequestOptions>,
	chat: Option<&'b ChatRequestOptions>,
}

impl<'a, 'b> ChatRequestOptionsSet<'a, 'b> {
	pub fn with_client_options(mut self, options: Option<&'a ChatRequestOptions>) -> Self {
		self.client = options;
		self
	}
	pub fn with_chat_options(mut self, options: Option<&'b ChatRequestOptions>) -> Self {
		self.chat = options;
		self
	}
}

impl ChatRequestOptionsSet<'_, '_> {
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
}

// endregion: --- ChatRequestOptionsSet
