//! This support model is for common constructs and utilities for all of the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::chat::{ChatRequestOptionsSet, MetaUsage};

// region:    --- StreamerChatOptions

#[derive(Debug, Default)]
pub struct StreamerOptions {
	pub capture_content: bool,
	pub capture_usage: bool,
}

impl From<ChatRequestOptionsSet<'_, '_>> for StreamerOptions {
	fn from(options_set: ChatRequestOptionsSet) -> Self {
		StreamerOptions {
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_usage: options_set.capture_usage().unwrap_or(false),
		}
	}
}

// endregion: --- StreamerChatOptions

// region:    --- Streamer Captured Data

#[derive(Debug, Default)]
pub struct StreamerCapturedData {
	pub usage: Option<MetaUsage>,
	pub content: Option<String>,
}

// endregion: --- Streamer Captured Data
