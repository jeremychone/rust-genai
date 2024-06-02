use crate::ollama::adapter::OllamaAdapter;
use crate::{ChatReq, ChatRes, Client, Result};
use async_trait::async_trait;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::ChatMessage;

#[async_trait]
impl Client for OllamaAdapter {
	async fn list_models(&self) -> Result<Vec<String>> {
		Ok(vec![])
	}

	async fn exec_chat(&self, model: &str, req: ChatReq) -> Result<ChatRes> {
		let raw_messages: Vec<ChatMessage> = req.messages.into_iter().map(ChatMessage::from).collect::<Vec<_>>();

		let raw_req = ChatMessageRequest::new(model.to_string(), raw_messages);

		let conn = &self.conn;
		let raw_res = conn.send_chat_messages(raw_req).await?;

		let res: ChatRes = raw_res.into();

		Ok(res)
	}

	async fn exec_chat_stream(&self, _model: &str, _req: ChatReq) -> Result<crate::ChatResStream> {
		todo!()
	}
}
