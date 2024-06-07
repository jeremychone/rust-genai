use crate::Result;
use derive_more::From;
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

#[derive(Debug, From)]
pub enum StreamEvent {
	Start,
	Chunk(StreamChunk),
	End(StreamEnd),
}

#[derive(Debug)]
pub struct StreamChunk {
	pub content: String,
}

#[derive(Debug, Default)]
pub struct StreamEnd {
	/// The optional captured full content
	/// NOTE: NOT SUPPORTED YET (always None for now)
	///       Probably allow to toggle this on at the client_config, adapter_config
	///       Also, in the chat API, might be nice ot have a Option<RequestOptions> with this flag
	pub captured_content: Option<String>,
}

// endregion: --- Chat Stream
