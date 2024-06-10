//! ChatRequestOptions is a struct that can be passed into the `client::exec_chat...` as the last argument
//! to customize the request behavior per call.
//! Note: Splitting it out of the `ChatRequest` object allows for better reusability of each component.
//!
//! IMPORTANT: These are not implemented yet, but here to show some of the directions and start having them part of the client APIs.

pub struct ChatRequestOptions {
	/// Will capture the `MetaUsage`
	/// - In the `ChatResponse` for `exec_chat`
	/// - In the `StreamEnd` of `StreamEvent::End(StreamEnd)` for `exec_chat_stream`
	pub capture_usage: Option<bool>,

	// -- For Stream only (for now, we flat them out)
	/// Tell the chat stream executor to capture and concatenate all of the text chunk
	/// to the last `StreamEvent::End(StreamEnd)` event as `StreamEnd.captured_content` (so, will be `Some(concatenated_chunks)`)
	pub capture_content: Option<bool>,
}
