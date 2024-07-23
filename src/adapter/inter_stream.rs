//! Internal Stream Event Types which serve as an intermediary between the Provider event and the genai stream event.
//!
//! This allows to eventually have flexibility if we want to capture event accross providers that does not need to
//! be reflected in the public ChatStream event.
//!
//! NOTE: This might be removed at some point as it might not be needed, and going directly to the genai stream.

use crate::chat::MetaUsage;

#[derive(Debug, Default)]
pub struct InterStreamEnd {
	// When `ChatOptions..capture_usage == true`
	pub captured_usage: Option<MetaUsage>,

	// When  `ChatOptions..capture_content == true`
	pub captured_content: Option<String>,
}

/// Intermediary StreamEvent
pub enum InterStreamEvent {
	Start,
	Chunk(String),
	End(InterStreamEnd),
}
