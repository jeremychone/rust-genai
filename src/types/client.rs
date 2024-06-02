use crate::{ChatRequest, ChatResponse, ChatStream, Result};
use async_trait::async_trait;

#[derive(Debug)]
pub enum ClientKind {
	OllamaRs,
	AsyncOpenAI,
}

#[async_trait]
pub trait Client {
	async fn list_models(&self) -> Result<Vec<String>>;

	async fn exec_chat(&self, model: &str, req: ChatRequest) -> Result<ChatResponse>;

	async fn exec_chat_stream(&self, model: &str, req: ChatRequest) -> Result<ChatStream>;
}
