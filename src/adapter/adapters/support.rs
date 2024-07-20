//! This support model is for common constructs and utilities for all of the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::chat::{ChatRequestOptionsSet, MetaUsage};
use crate::ModelInfo;

// region:    --- StreamerChatOptions

#[derive(Debug)]
pub struct StreamerOptions {
	pub capture_content: bool,
	pub capture_usage: bool,
	pub model_info: ModelInfo,
}

impl StreamerOptions {
	pub fn new(model_info: ModelInfo, options_set: ChatRequestOptionsSet<'_, '_>) -> Self {
		Self {
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_usage: options_set.capture_usage().unwrap_or(false),
			model_info,
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
