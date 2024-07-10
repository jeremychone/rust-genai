use crate::chat::ChatStream;

// region:    --- ChatResponse

#[derive(Debug, Clone, Default)]
pub struct ChatResponse {
	pub content: Option<String>,
	/// NOT SUPPORTED
	#[allow(unused)]
	pub meta_usage: Option<MetaUsage>,
}

// endregion: --- ChatResponse

// region:    --- ChatStreamResponse

pub struct ChatStreamResponse {
	pub stream: ChatStream,
}

// endregion: --- ChatStreamResponse

// region:    --- MetaUsage

/// IMPORTANT: This is **NOT SUPPORTED** for now. To show the API direction.
#[derive(Debug, Clone)]
pub struct MetaUsage {
	pub input_token: Option<i32>,
	pub output_token: Option<i32>,
	pub total_token: Option<i32>,
}

// endregion: --- MetaUsage
