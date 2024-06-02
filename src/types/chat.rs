use crate::Result;
use futures::Stream;
use std::pin::Pin;

// region:    --- ChatReq

#[derive(Debug, Clone)]
pub struct ChatReq {
	pub messages: Vec<ChatMsg>,
}

impl ChatReq {
	pub fn new(messages: Vec<ChatMsg>) -> Self {
		Self { messages }
	}
}

// endregion: --- ChatReq

// region:    --- ChatMsg

#[derive(Debug, Clone)]
pub struct ChatMsg {
	pub role: ChatRole,
	pub content: String,
	pub extra: Option<ChatMsgExtra>,
}

#[derive(Debug, Clone)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

#[derive(Debug, Clone)]
pub enum ChatMsgExtra {
	Tool(ToolExtra),
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct ToolExtra {
	tool_id: String,
}

/// Convenient constructors
impl ChatMsg {
	pub fn system(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::System,
			content: content.into(),
			extra: None,
		}
	}

	pub fn assistant(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::Assistant,
			content: content.into(),
			extra: None,
		}
	}

	pub fn user(content: impl Into<String>) -> Self {
		Self {
			role: ChatRole::User,
			content: content.into(),
			extra: None,
		}
	}
}

// endregion: --- ChatMsg

// region:    --- ChatRes

#[derive(Debug, Clone)]
pub struct ChatRes {
	pub response: Option<String>,
}

// endregion: --- ChatRes

// region:    --- ChatResStream

pub type ChatResStream = Pin<Box<dyn Stream<Item = Result<ChatResChunks>>>>;

pub type ChatResChunks = Vec<ChatResChunk>;

pub struct ChatResChunk {
	pub response: String,
}

// endregion: --- ChatResStream
