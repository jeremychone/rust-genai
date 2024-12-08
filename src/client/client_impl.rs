use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::{Client, Error, ModelIden, Result, ServiceTarget};

/// Public AI Functions
impl Client {
	/// Returns all the model names for a given adapter kind.
	///
	/// IMPORTANT:
	/// - Besides the Ollama adapter, this will only look at a hardcoded static list of names for now.
	/// - For Ollama, it will currently make a live request to the default host/port (http://localhost:11434/v1/).
	/// - This function will eventually change to either take an endpoint or have another function to allow a custom endpoint.
	///
	/// Notes:
	/// - Since genai only supports Chat for now, the adapter implementation should attempt to remove the non-chat models.
	/// - Later, as genai adds more capabilities, we will have a `model_names(adapter_kind, Option<&[Skill]>)`
	///   that will take a list of skills like (`ChatText`, `ChatImage`, `ChatFunction`, `TextToSpeech`, ...).
	pub async fn all_model_names(&self, adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let models = AdapterDispatcher::all_model_names(adapter_kind).await?;
		Ok(models)
	}

	/// Return the default model for a model_name str.
	/// This is used before
	pub fn default_model(&self, model_name: &str) -> Result<ModelIden> {
		// -- First get the default ModelInfo
		let adapter_kind = AdapterKind::from_model(model_name)?;
		let model_iden = ModelIden::new(adapter_kind, model_name);
		Ok(model_iden)
	}

	#[deprecated(note = "use `client.resolve_service_target(model_name)")]
	pub fn resolve_model_iden(&self, model_name: &str) -> Result<ModelIden> {
		let model = self.default_model(model_name)?;
		let target = self.config().resolve_service_target(model)?;
		Ok(target.model)
	}

	pub fn resolve_service_target(&self, model_name: &str) -> Result<ServiceTarget> {
		let model = self.default_model(model_name)?;
		self.config().resolve_service_target(model)
	}

	/// Executes a chat.
	pub async fn exec_chat(
		&self,
		model: &str,
		chat_req: ChatRequest,
		// options not implemented yet
		options: Option<&ChatOptions>,
	) -> Result<ChatResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let model = self.default_model(model)?;
		let target = self.config().resolve_service_target(model)?;
		let model = target.model.clone();

		let WebRequestData { headers, payload, url } =
			AdapterDispatcher::to_web_request_data(target, self.config(), ServiceType::Chat, chat_req, options_set)?;

		let web_res =
			self.web_client()
				.do_post(&url, &headers, payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model.clone(),
					webc_error,
				})?;

		let chat_res = AdapterDispatcher::to_chat_response(model, web_res)?;

		Ok(chat_res)
	}

	/// Executes a chat stream response.
	pub async fn exec_chat_stream(
		&self,
		model: &str,
		chat_req: ChatRequest, // options not implemented yet
		options: Option<&ChatOptions>,
	) -> Result<ChatStreamResponse> {
		let options_set = ChatOptionsSet::default()
			.with_chat_options(options)
			.with_client_options(self.config().chat_options());

		let model = self.default_model(model)?;
		let target = self.config().resolve_service_target(model)?;
		let model = target.model.clone();

		let WebRequestData { url, headers, payload } = AdapterDispatcher::to_web_request_data(
			target,
			self.config(),
			ServiceType::ChatStream,
			chat_req,
			options_set.clone(),
		)?;

		let reqwest_builder = self
			.web_client()
			.new_req_builder(&url, &headers, payload)
			.map_err(|webc_error| Error::WebModelCall {
				model_iden: model.clone(),
				webc_error,
			})?;

		let res = AdapterDispatcher::to_chat_stream(model, reqwest_builder, options_set)?;

		Ok(res)
	}
}
