use crate::providers::openai::OpenAIProvider;
use crate::{ChatRequest, ChatResponse, ChatStream, Client};
use crate::{Error, Result, StreamItem};
use async_openai::types as oa_types;
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl Client for OpenAIProvider {
	async fn list_models(&self) -> Result<Vec<String>> {
		Ok(vec![])
	}

	async fn exec_chat(&self, model: &str, req: ChatRequest) -> Result<ChatResponse> {
		// -- Get the async openai request
		let oa_req = into_oa_chat_req(model, req)?;

		// -- Exec the request
		let oa_chat_client = &self.conn().chat();
		let oa_res = oa_chat_client.create(oa_req).await?;

		let chat_res = oa_res.into();
		Ok(chat_res)
	}

	async fn exec_chat_stream(&self, model: &str, req: ChatRequest) -> Result<ChatStream> {
		// -- Get the async openai request
		let oa_req = into_oa_chat_req(model, req)?;

		// -- Exec the request
		let oa_chat_client = &self.conn().chat();
		let oa_res_stream = oa_chat_client.create_stream(oa_req).await?;

		let stream = oa_res_stream.map(|oa_stream_res| oa_stream_res.map(StreamItem::from).map_err(Error::from));

		Ok(ChatStream {
			stream: Box::pin(stream),
		})
	}
}

// region:    --- Support

fn into_oa_chat_req(model: &str, req: ChatRequest) -> Result<oa_types::CreateChatCompletionRequest> {
	let raw_messages = req
		.messages
		.into_iter()
		.map(oa_types::ChatCompletionRequestMessage::from)
		.collect::<Vec<_>>();

	let oa_req = oa_types::CreateChatCompletionRequestArgs::default()
		.max_tokens(512u16)
		.model(model)
		.n(1)
		.messages(raw_messages)
		.build()?;

	Ok(oa_req)
}

// endregion: --- Support
