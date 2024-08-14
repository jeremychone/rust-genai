use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::client::Client;
use crate::{Error, ModelIden, Result};

/// Public AI Functions
impl Client {
	/// Returns all the model names for a given adapter_kind
	/// Notes:
	/// - Since genai only supports Chat for now, the adapter implementation should try to remove the non-chat models
	/// - Later, as genai adds more capabilities, we will have a `model_names(adapter_kind, Option<&[Skill]>)`
	///   that will take a list of skills like (`ChatText`, `ChatImage`, `ChatFunction`, `TextToSpeech`, ...)
	pub async fn all_model_names(&self, adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let models = AdapterDispatcher::all_model_names(adapter_kind).await?;
		Ok(models)
	}

	/// Resolve the adapter kind for a given model name.
	/// Note: This does not use the `all_model_names` function to find a match, but instead relies on hardcoded matching rules.
	///       This strategy makes the library more flexible as it does not require updates
	///       when the AI Provider adds new models (assuming they follow a consistent naming pattern).
	///
	/// See [AdapterKind::from_model]
	///
	/// [AdapterKind::from_model]: crate::adapter::AdapterKind::from_model
	pub fn resolve_model_iden(&self, model_name: &str) -> Result<ModelIden> {
		// -- First get the default ModelInfo
		let adapter_kind = AdapterKind::from_model(model_name)?;
		let model_iden = ModelIden::new(adapter_kind, model_name);

		// -- Exec the eventual model_mapper
		let model_iden = if let Some(model_mapper) = self.config().model_mapper() {
			model_mapper
				.map_model(model_iden.clone())
				.map_err(|cause| Error::ModelMapperFailed { model_iden, cause })?
		} else {
			model_iden
		};

		Ok(model_iden)
	}

	/// Execute a chat
	pub async fn exec_chat(
		&self,
		model: &str,
		chat_req: ChatRequest,
		// options not implemented yet
		options: Option<&ChatOptions>,
	) -> Result<ChatResponse> {
		let model_iden = self.resolve_model_iden(model)?;

		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let WebRequestData { headers, payload, url } = AdapterDispatcher::to_web_request_data(
			model_iden.clone(),
			self.config(),
			ServiceType::Chat,
			chat_req,
			options_set,
		)?;

		let web_res =
			self.web_client()
				.do_post(&url, &headers, payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model_iden.clone(),
					webc_error,
				})?;

		let chat_res = AdapterDispatcher::to_chat_response(model_iden, web_res)?;

		Ok(chat_res)
	}

	pub async fn exec_chat_stream(
		&self,
		model: &str,
		chat_req: ChatRequest, // options not implemented yet
		options: Option<&ChatOptions>,
	) -> Result<ChatStreamResponse> {
		let model_iden = self.resolve_model_iden(model)?;

		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let WebRequestData { url, headers, payload } = AdapterDispatcher::to_web_request_data(
			model_iden.clone(),
			self.config(),
			ServiceType::ChatStream,
			chat_req,
			options_set.clone(),
		)?;

		let reqwest_builder = self
			.web_client()
			.new_req_builder(&url, &headers, payload)
			.map_err(|webc_error| Error::WebModelCall {
				model_iden: model_iden.clone(),
				webc_error,
			})?;

		let res = AdapterDispatcher::to_chat_stream(model_iden, reqwest_builder, options_set)?;

		Ok(res)
	}
}
