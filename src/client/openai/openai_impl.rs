use crate::client::openai::OpenAIClient;
use crate::ChatMsg;
use crate::ChatReq;
use crate::ChatRes;
use crate::ChatResStream;
use crate::ChatRole;
use crate::Client;
use crate::Result;
use crate::{GenReq, GenRes, GenResStream};
use async_openai::config::OpenAIConfig;
use async_openai::types::ChatCompletionRequestMessage;
use async_openai::types::ChatCompletionRequestUserMessage;
use async_openai::types::Role;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[async_trait]
impl Client for OpenAIClient {
	async fn list_models(&self) -> Result<Vec<String>> {
		Ok(vec![])
	}
	async fn exec_chat(&self, req: ChatReq) -> Result<ChatRes> {
		let raw_client = self.conn.lock().unwrap();
		let chat_client = raw_client.chat();

		let model = req.model;
		let raw_messages = req.messages.into_iter().map(ChatCompletionRequestMessage::from);

		todo!()
	}

	async fn exec_chat_stream(&self, req: ChatReq) -> Result<ChatResStream> {
		todo!()
	}
}

// region:    --- Types From

impl From<ChatRole> for Role {
	fn from(chat_role: ChatRole) -> Self {
		match chat_role {
			ChatRole::System => Role::System,
			ChatRole::User => Role::User,
			ChatRole::Assistant => Role::Assistant,
			ChatRole::Tool => Role::Tool,
			// TODO: need to decide what to do with function
		}
	}
}

impl From<ChatMsg> for ChatCompletionRequestMessage {
	fn from(chat_msg: ChatMsg) -> Self {
		let role = chat_msg.role.into();
		let content = chat_msg.content;

		match chat_msg.extra {
			None => ChatCompletionRequestUserMessage {
				content: content.into(),
				role,
				name: None,
			}
			.into(),
			_ => todo!("chat_msg other not supported yet"),
		}
	}
}

// endregion: --- Types From
