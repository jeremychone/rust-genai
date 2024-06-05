use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatResponse, ChatStream};
use crate::client::Client;
use crate::Result;

/// Public AI Functions
impl Client {
	pub async fn list_models(&self, _adapter_kind: AdapterKind) -> Result<Vec<String>> {
		todo!()
	}

	pub async fn exec_chat(&self, model: &str, chat_req: ChatRequest) -> Result<ChatResponse> {
		let adapter_kind = AdapterKind::from_model(model)?;

		let WebRequestData { headers, payload } =
			AdapterDispatcher::to_web_request_data(adapter_kind, model, chat_req, false)?;

		let url = AdapterDispatcher::get_service_url(adapter_kind, ServiceType::Chat);

		let web_res = self.web_client().do_post(&url, &headers, payload).await?;

		let chat_res = AdapterDispatcher::to_chat_response(adapter_kind, web_res)?;

		Ok(chat_res)
	}

	pub async fn exec_chat_stream(&self, model: &str, chat_req: ChatRequest) -> Result<ChatStream> {
		let adapter_kind = AdapterKind::from_model(model)?;

		let WebRequestData { headers, payload } =
			AdapterDispatcher::to_web_request_data(adapter_kind, model, chat_req, true)?;

		let url = AdapterDispatcher::get_service_url(adapter_kind, ServiceType::Chat);

		let event_source = self.web_client().do_post_stream(&url, &headers, payload).await?;

		AdapterDispatcher::to_chat_stream(adapter_kind, event_source)
	}
}
