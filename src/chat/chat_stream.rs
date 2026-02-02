use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{MessageContent, ToolCall, Usage};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

type InterStreamType = Pin<Box<dyn Stream<Item = crate::Result<InterStreamEvent>> + Send>>;

/// A stream of chat events produced by a streaming chat request.
pub struct ChatStream {
	inter_stream: InterStreamType,
}

impl ChatStream {
	pub(crate) fn new(inter_stream: InterStreamType) -> Self {
		ChatStream { inter_stream }
	}

	pub(crate) fn from_inter_stream<T>(inter_stream: T) -> Self
	where
		T: Stream<Item = crate::Result<InterStreamEvent>> + Send + 'static,
	{
		let boxed_stream: InterStreamType = Box::pin(inter_stream);
		ChatStream::new(boxed_stream)
	}
}

// region:    --- Stream Impl

impl Stream for ChatStream {
	type Item = crate::Result<ChatStreamEvent>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		loop {
			match Pin::new(&mut this.inter_stream).poll_next(cx) {
				Poll::Ready(Some(Ok(event))) => {
					let chat_event = match event {
						InterStreamEvent::Start => ChatStreamEvent::Start,
						InterStreamEvent::Chunk(content) => ChatStreamEvent::Chunk(StreamChunk { content }),
						InterStreamEvent::ReasoningChunk(content) => {
							ChatStreamEvent::ReasoningChunk(StreamChunk { content })
						}
						InterStreamEvent::ToolCallChunk(tool_call) => {
							ChatStreamEvent::ToolCallChunk(ToolChunk { tool_call })
						}
						InterStreamEvent::ThoughtSignatureChunk(_signature) => {
							// Thought signatures are internal metadata, not streamed to users
							// Skip this event and continue polling
							continue;
						}
						InterStreamEvent::End(inter_end) => ChatStreamEvent::End(inter_end.into()),
					};
					return Poll::Ready(Some(Ok(chat_event)));
				}
				Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}

// endregion: --- Stream Impl

// region:    --- ChatStreamEvent

/// Provider-agnostic chat events returned by `Client::exec()` when streaming.
#[derive(Debug, Serialize, Deserialize)]
pub enum ChatStreamEvent {
	/// Emitted once at the start of the stream.
	Start,

	/// Assistant content chunk (text).
	Chunk(StreamChunk),

	/// Reasoning content chunk.
	ReasoningChunk(StreamChunk),

	/// Tool-call chunk.
	ToolCallChunk(ToolChunk),

	/// End of stream.
	/// May include captured usage and/or content when enabled via `ChatOptions`.
	End(StreamEnd),
}

/// Content of `ChatStreamEvent::Chunk`.
/// Currently text only.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunk {
	/// Text content.
	pub content: String,
}

/// Content of `ChatStreamEvent::ToolCallChunk`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChunk {
	/// The tool call.
	pub tool_call: ToolCall,
}

/// Terminal event data with optionally captured usage and content.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StreamEnd {
	/// Captured usage if `ChatOptions.capture_usage` is enabled.
	pub captured_usage: Option<Usage>,

	/// Captured final content (text and tool calls) if `ChatOptions.capture_content`
	/// or `capture_tool_calls` is enabled.
	/// Note: Since 0.4.0 this includes tool calls as well (for API symmetry with `ChatResponse`);
	///       use `.captured_tool_calls()` or `.captured_texts()`.
	pub captured_content: Option<MessageContent>,

	/// Captured reasoning content if `ChatOptions.capture_reasoning` is enabled.
	pub captured_reasoning_content: Option<String>,
}

impl From<InterStreamEnd> for StreamEnd {
	fn from(inter_end: InterStreamEnd) -> Self {
		let captured_text_content = inter_end.captured_text_content;
		let captured_tool_calls = inter_end.captured_tool_calls;

		// -- create public captured_content
		let mut captured_content: Option<MessageContent> = None;
		if let Some(captured_text_content) = captured_text_content {
			// This `captured_text_content` is the concatenation of all text chunks received.
			captured_content = Some(MessageContent::from_text(captured_text_content));
		}
		if let Some(captured_tool_calls) = captured_tool_calls {
			if let Some(existing_content) = &mut captured_content {
				existing_content.extend(MessageContent::from_tool_calls(captured_tool_calls));
			} else {
				// This `captured_tool_calls` is the concatenation of all tool call chunks received.
				captured_content = Some(MessageContent::from_tool_calls(captured_tool_calls));
			}
		}

		// -- Return result
		StreamEnd {
			captured_usage: inter_end.captured_usage,
			captured_content,
			captured_reasoning_content: inter_end.captured_reasoning_content,
		}
	}
}

/// Getters
impl StreamEnd {
	/// Returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_first_text(&self) -> Option<&str> {
		let captured_content = self.captured_content.as_ref()?;
		captured_content.first_text()
	}

	/// Consumes `self` and returns the first captured text, if any.
	/// This is the concatenation of all streamed text chunks.
	pub fn captured_into_first_text(self) -> Option<String> {
		let captured_content = self.captured_content?;
		captured_content.into_first_text()
	}

	/// Returns all captured text segments, if any.
	pub fn captured_texts(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.texts())
	}

	/// Consumes `self` and returns all captured text segments, if any.
	pub fn into_texts(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_texts())
	}

	/// Returns all captured tool calls, if any.
	pub fn captured_tool_calls(&self) -> Option<Vec<&ToolCall>> {
		let captured_content = self.captured_content.as_ref()?;
		Some(captured_content.tool_calls())
	}

	/// Consumes `self` and returns all captured tool calls, if any.
	pub fn captured_into_tool_calls(self) -> Option<Vec<ToolCall>> {
		let captured_content = self.captured_content?;
		Some(captured_content.into_tool_calls())
	}
}

// endregion: --- ChatStreamEvent
