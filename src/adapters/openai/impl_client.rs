use crate::adapters::openai::{AsyncOaClient, OpenAIAdapter};
use crate::openai::OpenAIAdapterConfig;
use crate::Result;
use crate::{ChatReq, ChatRes, ChatResStream, Client};
use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::types::{ChatCompletionRequestMessage, CreateChatCompletionRequestArgs};
use async_openai::Client as AsyncOpenAIClient;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[async_trait]
impl Client for OpenAIAdapter {
	async fn list_models(&self) -> Result<Vec<String>> {
		Ok(vec![])
	}

	async fn exec_chat(&self, model: &str, req: ChatReq) -> Result<ChatRes> {
		let raw_messages = req
			.messages
			.into_iter()
			.map(ChatCompletionRequestMessage::from)
			.collect::<Vec<_>>();

		let raw_req = CreateChatCompletionRequestArgs::default()
			.max_tokens(512u16)
			.model(model)
			.messages(raw_messages)
			.build()?;

		let conn = self.conn.lock().await;
		let chat_client = conn.chat();
		let raw_res = chat_client.create(raw_req).await?;

		let chat_res = raw_res.into();
		Ok(chat_res)
	}

	async fn exec_chat_stream(&self, _model: &str, _req: ChatReq) -> Result<ChatResStream> {
		todo!()
	}
}
