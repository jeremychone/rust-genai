use crate::Result;
use futures::Stream;
use std::pin::Pin;

// region:    --- Chat Request Types

#[derive(Debug)]
pub enum ChatRole {
	System,
	User,
	Assistant,
	Tool,
}

pub struct ChatReq {
	pub model: String,
	pub messages: Vec<ChatMsg>,
}

pub struct ChatMsg {
	pub role: ChatRole,
	pub content: String,
	pub extra: Option<ChatMsgExtra>,
}

pub enum ChatMsgExtra {
	Tool(ToolExtra),
}

pub struct ToolExtra {
	tool_id: String,
}

// endregion: --- Chat Request Types

// region:    --- ChatRes

#[derive(Debug, Clone)]
pub struct ChatRes {
	pub response: String,
}

// endregion: --- ChatRes

// region:    --- ChatResStream

pub type ChatResStream = Pin<Box<dyn Stream<Item = Result<ChatResChunks>>>>;

pub type ChatResChunks = Vec<ChatResChunk>;

pub struct ChatResChunk {
	pub response: String,
}

// endregion: --- ChatResStream
