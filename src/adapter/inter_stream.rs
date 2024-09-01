//! Internal Stream Event Types which serve as an intermediary between the Provider event and the genai stream event.
//!
//! This allows to eventually have flexibility if we want to capture event across providers that does not need to
//! be reflected in the public ChatStream event.
//!
//! NOTE: This might be removed at some point as it might not be needed, and going directly to the genai stream.

use serde::{Deserialize, Serialize};

use crate::chat::MetaUsage;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InterStreamEnd {
	// When `ChatOptions..capture_usage == true`
	pub captured_usage: Option<MetaUsage>,

	// When  `ChatOptions..capture_content == true`
	pub captured_content: Option<String>,
}

/// Intermediary StreamEvent
#[derive(Debug, Serialize, Deserialize)]
pub enum InterStreamEvent {
	Start,
	Chunk(String),
	End(InterStreamEnd),
}
