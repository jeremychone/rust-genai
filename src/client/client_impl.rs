use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStreamResponse};
use crate::client::Client;
use crate::{ConfigSet, Result};

/// Public AI Functions
impl Client {
	pub async fn list_models(&self, _adapter_kind: AdapterKind) -> Result<Vec<String>> {
		todo!()
	}

	pub async fn exec_chat(&self, model: &str, chat_req: ChatRequest) -> Result<ChatResponse> {
		let adapter_kind = AdapterKind::from_model(model)?;

		let adapter_config = self
			.custom_adapter_config(adapter_kind)
			.unwrap_or_else(|| AdapterDispatcher::default_adapter_config(adapter_kind));

		let config_set = ConfigSet::new(self.config(), adapter_config);

		let WebRequestData { headers, payload, url } =
			AdapterDispatcher::to_web_request_data(adapter_kind, &config_set, model, chat_req, ServiceType::Chat)?;

		let web_res = self.web_client().do_post(&url, &headers, payload).await?;

		let chat_res = AdapterDispatcher::to_chat_response(adapter_kind, web_res)?;

		Ok(chat_res)
	}

	pub async fn exec_chat_stream(&self, model: &str, chat_req: ChatRequest) -> Result<ChatStreamResponse> {
		let adapter_kind = AdapterKind::from_model(model)?;

		let adapter_config = self
			.custom_adapter_config(adapter_kind)
			.unwrap_or_else(|| AdapterDispatcher::default_adapter_config(adapter_kind));

		let config_set = ConfigSet::new(self.config(), adapter_config);

		let WebRequestData { url, headers, payload } = AdapterDispatcher::to_web_request_data(
			adapter_kind,
			&config_set,
			model,
			chat_req,
			ServiceType::ChatStream,
		)?;

		let reqwest_builder = self.web_client().new_req_builder(&url, &headers, payload)?;

		AdapterDispatcher::to_chat_stream(adapter_kind, reqwest_builder)
	}
}
