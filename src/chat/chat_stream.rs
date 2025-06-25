use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{MessageContent, ToolCall, Usage};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::task::{Context, Poll};

type InterStreamType = Pin<Box<dyn Stream<Item = crate::Result<InterStreamEvent>> + Send>>;

/// ChatStream is a Rust Future Stream that iterates through the events of a chat stream request.
pub struct ChatStream {
	inter_stream: InterStreamType,
}

impl ChatStream {
	pub(crate) fn new(inter_stream: InterStreamType) -> Self {
		ChatStream { inter_stream }
	}

	pub(crate) fn from_inter_stream<T>(inter_stream: T) -> Self
	where
		T: Stream<Item = crate::Result<InterStreamEvent>> + Send + Unpin + 'static,
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
					InterStreamEvent::End(inter_end) => ChatStreamEvent::End(inter_end.into()),
				};
				Poll::Ready(Some(Ok(chat_event)))
			}
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

// endregion: --- Stream Impl

// region:    --- ChatStreamEvent

/// The normalized chat stream event for any provider when calling `Client::exec`.
#[derive(Debug, Serialize, Deserialize)]
pub enum ChatStreamEvent {
	/// Represents the start of the stream. The first event.
	Start,

	/// Represents each content chunk. Currently, it only contains text content.
	Chunk(StreamChunk),

	/// Represents the reasoning_content chunk.
	ReasoningChunk(StreamChunk),

	/// Represents tool call chunks.
	ToolCallChunk(ToolChunk),

	/// Represents the end of the stream.
	/// It will have the `.captured_usage` and `.captured_content` if specified in the `ChatOptions`.
	End(StreamEnd),
}

/// Chunk content of the `ChatStreamEvent::Chunk` variant.
/// For now, it only contains text.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunk {
	/// The content text.
	pub content: String,
}

/// Tool call chunk content of the `ChatStreamEvent::ToolCallChunk` variant.
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolChunk {
	/// The tool call.
	pub tool_call: ToolCall,
}

/// StreamEnd content, with the eventual `.captured_usage` and `.captured_content`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StreamEnd {
	/// The eventual captured usage metadata.
	/// Note: This requires the ChatOptions `capture_usage` flag to be set to true.
	pub captured_usage: Option<Usage>,

	/// The eventual captured full content.
	/// Note: This requires the ChatOptions `capture_content` or `capture_tool_calls` flags to be set to true.
	/// Note: Since 0.4.0 this will have the tool calls as well (for API symmetry with the ChatRespone), call `.captured_tool_calls()` or `.captured_texts()` ...
	pub captured_content: Option<Vec<MessageContent>>,

	/// The eventual captured
	/// Note: This requires the ChatOptions `capture_reasoning` flag to be set to true.
	pub captured_reasoning_content: Option<String>,
}

impl From<InterStreamEnd> for StreamEnd {
	fn from(inter_end: InterStreamEnd) -> Self {
		let captured_text_content = inter_end.captured_text_content;
		let captured_tool_calls = inter_end.captured_tool_calls;

		// -- create public captured_content
		let mut captured_content: Option<Vec<MessageContent>> = None;
		if let Some(captured_text_content) = captured_text_content {
			// This `captured_text_content` is the concatenation of all text chunks received.
			captured_content = Some(vec![MessageContent::Text(captured_text_content)]);
		}
		if let Some(captured_tool_calls) = captured_tool_calls {
			if let Some(existing_content) = &mut captured_content {
				existing_content.push(MessageContent::ToolCalls(captured_tool_calls));
			} else {
				// This `captured_tool_calls` is the concatenation of all tool call chunks received.
				captured_content = Some(vec![MessageContent::ToolCalls(captured_tool_calls)]);
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
	/// Returns a reference to the first captured text content if available.
	/// This is the concatenation of all text chunks received during the stream.
	pub fn captured_first_text(&self) -> Option<&str> {
		let captured_content = self.captured_content.as_ref()?;

		for content_item in captured_content {
			if let MessageContent::Text(content) = content_item {
				return Some(content);
			}
		}
		None
	}

	/// Consumes the `StreamEnd` and returns the first captured text content if available.
	/// This is the concatenation of all text chunks received during the stream.
	pub fn captured_into_first_text(self) -> Option<String> {
		let captured_content = self.captured_content?;

		for content_item in captured_content {
			if let MessageContent::Text(content) = content_item {
				return Some(content);
			}
		}
		None
	}

	/// Returns a vector of references to all captured text content parts.
	/// Each element in the vector represents the concatenation of text chunks received consecutively.
	pub fn captured_texts(&self) -> Option<Vec<&str>> {
		let captured_content = self.captured_content.as_ref()?;

		let mut all_texts = Vec::new();
		for content_item in captured_content {
			if let MessageContent::Text(content) = content_item {
				all_texts.push(content.as_str());
			}
		}
		Some(all_texts)
	}

	/// Consumes the `StreamEnd` and returns a vector of all captured text content parts.
	/// Each element in the vector represents the concatenation of text chunks received consecutively.
	pub fn into_texts(self) -> Option<Vec<String>> {
		let captured_content = self.captured_content?;

		let mut all_texts = Vec::new();

		for content_item in captured_content {
			if let MessageContent::Text(content) = content_item {
				all_texts.push(content);
			}
		}
		Some(all_texts)
	}

	/// Returns a vector of references to all captured tool calls.
	/// This is the concatenation of all tool call chunks received during the stream.
	pub fn captured_tool_calls(&self) -> Option<Vec<&ToolCall>> {
		let captured_content = self.captured_content.as_ref()?;

		let mut all_tool_calls: Vec<&ToolCall> = Vec::new();
		for content_item in captured_content {
			if let MessageContent::ToolCalls(tool_calls) = content_item {
				for tool_call in tool_calls {
					all_tool_calls.push(tool_call);
				}
			}
		}

		Some(all_tool_calls)
	}

	/// Consumes the `StreamEnd` and returns a vector of all captured tool calls.
	/// This is the concatenation of all tool call chunks received during the stream.
	pub fn captured_into_tool_calls(self) -> Option<Vec<ToolCall>> {
		let captured_content = self.captured_content?;

		let mut all_tool_calls: Vec<ToolCall> = Vec::new();
		for content_item in captured_content {
			if let MessageContent::ToolCalls(tool_calls) = content_item {
				for tool_call in tool_calls {
					all_tool_calls.push(tool_call);
				}
			}
		}

		Some(all_tool_calls)
	}
}

// endregion: --- ChatStreamEvent
