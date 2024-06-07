use crate::Result;
use futures::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct ChatResponse {
	pub content: Option<String>,
}

// region:    --- Chat Stream

type StreamType = Pin<Box<dyn Stream<Item = Result<StreamEvent>>>>;

pub struct ChatStream {
	pub stream: StreamType,
}

pub struct StreamEvent {
	pub content: Option<String>,
}

// endregion: --- Chat Stream
