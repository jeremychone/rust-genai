//! This support model is for common constructs and utilities for all of the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::chat::{ChatOptionsSet, MetaUsage};
use crate::ModelIden;

// region:    --- StreamerChatOptions

#[derive(Debug)]
pub struct StreamerOptions {
	pub capture_content: bool,
	pub capture_usage: bool,
	pub model_iden: ModelIden,
}

impl StreamerOptions {
	pub fn new(model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_usage: options_set.capture_usage().unwrap_or(false),
			model_iden,
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