//! ChatRequestOptions is a struct that can be passed into the `client::exec_chat...` as the last argument
//! to customize the request behavior per call.
//! Note: Splitting it out of the `ChatRequest` object allows for better reusability of each component.
//!
//! IMPORTANT: These are not implemented yet, but here to show some of the directions and start having them part of the client APIs.

#[derive(Debug, Clone, Default)]
pub struct ChatRequestOptions {
	// -- Adapter/Provider request property
	//
	/// Will be set for this request if Adapter/providers supports it.
	pub temperature: Option<f32>,

	/// Will be set of this request if Adaptper/provider supports it.
	pub max_tokens: Option<u32>,

	// -- `genai` request runtime options
	//
	/// - In the `ChatResponse` for `exec_chat`
	/// - In the `StreamEnd` of `StreamEvent::End(StreamEnd)` for `exec_chat_stream`
	/// > Note: Will capture the `MetaUsage`
	pub capture_usage: Option<bool>,

	// -- For Stream only (for now, we flat them out)
	//
	/// Tell the chat stream executor to capture and concatenate all of the text chunk
	/// to the last `StreamEvent::End(StreamEnd)` event as `StreamEnd.captured_content` (so, will be `Some(concatenated_chunks)`)
	pub capture_content: Option<bool>,
}

/// Setters (builder styles)
impl ChatRequestOptions {
	/// Set the `temperature` for this request.
	pub fn with_temperature(mut self, value: f32) -> Self {
		self.temperature = Some(value);
		self
	}

	/// Set the `max_tokens` for this request.
	pub fn with_max_tokens(mut self, value: u32) -> Self {
		self.max_tokens = Some(value);
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
