use crate::{ChatReq, ChatRes, ChatResStream, Result};
use async_trait::async_trait;

#[derive(Debug)]
pub enum ClientKind {
	Ollama,
	OpenAI,
}

#[async_trait]
pub trait Client {
	async fn list_models(&self) -> Result<Vec<String>>;

	async fn exec_chat(&self, model: &str, req: ChatReq) -> Result<ChatRes>;

	async fn exec_chat_stream(&self, model: &str, req: ChatReq) -> Result<ChatResStream>;
}
