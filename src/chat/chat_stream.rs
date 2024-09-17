use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{MessageContent, MetaUsage};
use derive_more::From;
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
#[derive(Debug, From, Serialize, Deserialize)]
pub enum ChatStreamEvent {
	/// Represents the start of the stream. First event.
	Start,

	/// Represents each chunk response. Currently only contains text content.
	Chunk(StreamChunk),

	/// Represents the end of the stream.
	/// Will have the `.captured_usage` and `.captured_content` if specified in the `ChatOptions`.
	End(StreamEnd),
}

/// Chunk content of the `ChatStreamEvent::Chunk` variant.
/// For now, it only contains text.
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamChunk {
	/// The content text.
	pub content: String,
}

/// StreamEnd content, with the eventual `.captured_usage` and `.captured_content`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StreamEnd {
	/// The eventual captured UsageMeta.
	pub captured_usage: Option<MetaUsage>,

	/// The optional captured full content.
	pub captured_content: Option<MessageContent>,
}

impl From<InterStreamEnd> for StreamEnd {
	fn from(inter_end: InterStreamEnd) -> Self {
		StreamEnd {
			captured_usage: inter_end.captured_usage,
			captured_content: inter_end.captured_content.map(MessageContent::from),
		}
	}
}

// endregion: --- ChatStreamEvent
