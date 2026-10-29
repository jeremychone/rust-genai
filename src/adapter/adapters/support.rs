//! This support module is for common constructs and utilities for all the adapter implementations.
//! It should be private to the `crate::adapter::adapters` module.

use crate::ModelIden;
use crate::chat::{ChatOptionsSet, FrameCtx, ToolResponse, Usage};
use crate::resolver::AuthData;
use crate::webc::FrameTap;
use crate::{Error, Result};

pub fn get_api_key(auth: AuthData, model: &ModelIden) -> Result<String> {
	auth.single_key_value().map_err(|resolver_error| Error::Resolver {
		model_iden: model.clone(),
		resolver_error,
	})
}

/// Builds the tap that feeds a user `ChatFrameSink`, when one is configured.
pub fn new_frame_tap(model_iden: &ModelIden, options_set: &ChatOptionsSet<'_, '_>) -> Option<FrameTap> {
	let sink = options_set.raw_frame_sink()?;
	Some(FrameTap::new(sink, FrameCtx::new(model_iden.clone())))
}

/// The text of a tool response, for providers whose tool results accept only text.
/// Binary parts are dropped, with a warning.
pub fn tool_response_text_only(model_iden: &ModelIden, tool_response: &ToolResponse) -> String {
	warn_dropped_tool_response_binaries(model_iden, tool_response, tool_response.binaries().len());
	tool_response.text_content()
}

/// Warns that `dropped` binary parts of a tool response are not sent to the provider.
pub fn warn_dropped_tool_response_binaries(model_iden: &ModelIden, tool_response: &ToolResponse, dropped: usize) {
	if dropped > 0 {
		tracing::warn!(
			model = %model_iden,
			call_id = %tool_response.call_id,
			dropped,
			"tool response binary parts are not supported by this provider for tool results; sending only the text parts"
		);
	}
}

// region:    --- StreamerChatOptions

#[derive(Debug)]
pub struct StreamerOptions {
	pub capture_usage: bool,
	pub capture_reasoning_content: bool,
	pub capture_content: bool,
	pub capture_tool_calls: bool,
	pub model_iden: ModelIden,
}

impl StreamerOptions {
	pub fn new(model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			capture_usage: options_set.capture_usage().unwrap_or(false),
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_reasoning_content: options_set.capture_reasoning_content().unwrap_or(false),
			capture_tool_calls: options_set.capture_tool_calls().unwrap_or(false),
			model_iden,
		}
	}
}

// endregion: --- StreamerChatOptions

// region:    --- Streamer Captured Data

#[derive(Debug, Default)]
pub struct StreamerCapturedData {
	pub usage: Option<Usage>,
	pub stop_reason: Option<String>,
	pub content: Option<String>,
	pub reasoning_content: Option<String>,
	pub tool_calls: Option<Vec<crate::chat::ToolCall>>,
	pub thought_signatures: Option<Vec<String>>,
}

// endregion: --- Streamer Captured Data
