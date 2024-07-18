#![allow(unused)] // For test support
//! Some support utilities for the tests
//! Note: Must be imported in each test files

pub mod common_tests;

use genai::chat::{ChatMessage, ChatRequest, ChatStream, ChatStreamEvent, StreamEnd};
use tokio_stream::StreamExt;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

/// A macro to get the value of an `Option` field from a struct, returning an error if the field is `None`.
///
/// # Arguments
///
/// * `$expr` - The expression representing the struct and its field.
///
/// # Example
///
/// ```rust
/// let meta_usage = get_option_value!(stream_end.meta_usage);
/// ```
///
/// This macro expands to:
///
/// ```rust
/// let meta_usage = stream_end.meta_usage.ok_or("Should have meta_usage")?;
/// ```
#[macro_export]
macro_rules! get_option_value {
	($struct:ident.$field:ident) => {
		$struct.$field.ok_or(concat!("Should have ", stringify!($field)))?
	};
}

pub async fn extract_stream_end(mut chat_stream: ChatStream) -> Result<StreamEnd> {
	let mut stream_end: Option<StreamEnd> = None;
	while let Some(Ok(stream_event)) = chat_stream.next().await {
		if let ChatStreamEvent::End(end_evt) = stream_event {
			stream_end = Some(end_evt);
			break;
		}
	}

	stream_end.ok_or("Should have a StreamEnd event".into())
}

// region:    --- Seeder

pub fn seed_chat_req_simple() -> ChatRequest {
	ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is sky blue?"),
	])
}

// endregion: --- Seeder
