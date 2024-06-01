// region:    --- Modules

mod openai;

pub use openai::*;

// endregion: --- Modules

use crate::{ChatReq, ChatRes, ChatResStream, GenReq, GenRes, GenResStream, Result};
use async_trait::async_trait;

pub enum ClientKind {
	Ollama,
	Openai,
}

#[async_trait]
pub trait Client {
	async fn list_models(&self) -> Result<Vec<String>>;

	async fn exec_chat(&self, req: ChatReq) -> Result<ChatRes>;

	async fn exec_chat_stream(&self, req: ChatReq) -> Result<ChatResStream>;
}
