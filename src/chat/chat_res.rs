use crate::chat::ChatStream;

// region:    --- ChatResponse

#[derive(Debug, Clone)]
pub struct ChatResponse {
	pub content: Option<String>,
}

// endregion: --- ChatResponse

// region:    --- ChatStreamResponse

pub struct ChatStreamResponse {
	pub stream: ChatStream,
}

// endregion: --- ChatStreamResponse
