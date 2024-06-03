use crate::ollama::provider::OllamaProvider;
use crate::{ChatRequest, ChatResponse, ChatStream, Client, ClientKind, Error, Result, StreamItem};
use async_trait::async_trait;
use futures::StreamExt;
use ollama_rs::generation::chat as ol_chat;
use ollama_rs::generation::chat::request as ol_req;

#[async_trait]
impl Client for OllamaProvider {
	async fn list_models(&self) -> Result<Vec<String>> {
		Ok(vec![])
	}

	async fn exec_chat(&self, model: &str, req: ChatRequest) -> Result<ChatResponse> {
		let ol_req = into_ol_chat_req(model, req)?;

		let ol_client = &self.conn;
		let ol_res = ol_client.send_chat_messages(ol_req).await?;

		let res: ChatResponse = ol_res.into();

		Ok(res)
	}

	async fn exec_chat_stream(&self, model: &str, req: ChatRequest) -> Result<crate::ChatStream> {
		// -- Get the ollama request
		let ol_req = into_ol_chat_req(model, req)?;

		// -- Exec the request
		let ol_client = &self.conn;
		// IMPORTANT:
		let ol_res_stream = ol_client.send_chat_messages_stream(ol_req).await?;

		let stream = ol_res_stream.map(|ol_stream_res| {
			let chat_item = ol_stream_res.map(StreamItem::from);
			// Note: here, ollama `ol_stream_res` is a result with () error, so, add some context
			// TODO: need to create the custom
			chat_item.map_err(|_| {
				Error::provider_connector(ClientKind::OllamaRs, "() ollama-rs error (error on a stream response)")
			})
		});

		Ok(ChatStream {
			stream: Box::pin(stream),
		})
	}
}

fn into_ol_chat_req(model: &str, req: ChatRequest) -> Result<ol_req::ChatMessageRequest> {
	let ol_messages: Vec<ol_chat::ChatMessage> =
		req.messages.into_iter().map(ol_chat::ChatMessage::from).collect::<Vec<_>>();

	let ol_req = ol_req::ChatMessageRequest::new(model.to_string(), ol_messages);

	Ok(ol_req)
}
