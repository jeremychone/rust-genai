use crate::adapter::{AdapterDispatcher, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptions, ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptions, EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::AuthData;
use crate::{Client, Error, ModelIden, Result, ServiceTarget};

/// High-level client APIs.
impl Client {
	/// Lists model names for the given adapter.
	///
	/// Notes:
	///
	/// - Non-Ollama adapters use a static list.
	///
	/// - Ollama queries the resolved host (defaults to http://localhost:11434/v1/).
	///
	/// - May evolve to accept a custom endpoint.
	///
	/// - For most adapters, names also drive AdapterKind detection (see [`AdapterKind`]).
	///
	/// - Adapters should filter non-chat models until more skills are supported.
	///   Future: `model_names(adapter_kind, Option<&[Skill]>)`.
	pub async fn all_model_names(&self, adapter_kind: AdapterKind) -> Result<Vec<String>> {
		if adapter_kind == AdapterKind::Ollama {
			let placeholder_model = ModelIden::new(adapter_kind, "__ollama_list_models__");
			let config = self.config();

			let auth = if let Some(auth_resolver) = config.auth_resolver() {
				auth_resolver
					.resolve(placeholder_model.clone())
					.await
					.map_err(|resolver_error| Error::Resolver {
						model_iden: placeholder_model.clone(),
						resolver_error,
					})?
					.unwrap_or_else(|| AdapterDispatcher::default_auth(adapter_kind))
			} else {
				AdapterDispatcher::default_auth(adapter_kind)
			};

			let mut service_target = ServiceTarget {
				endpoint: AdapterDispatcher::default_endpoint(adapter_kind),
				auth,
				model: placeholder_model.clone(),
			};

			if let Some(resolver) = config.service_target_resolver() {
				service_target = resolver
					.resolve(service_target)
					.await
					.map_err(|resolver_error| Error::Resolver {
						model_iden: placeholder_model.clone(),
						resolver_error,
					})?;
			}

			let models =
				AdapterDispatcher::all_model_names_with_target(adapter_kind, service_target, self.web_client()).await?;

			return Ok(models);
		}

		let models = AdapterDispatcher::all_model_names(adapter_kind).await?;
		Ok(models)
	}

	/// Builds a ModelIden by inferring AdapterKind from the model name.
	pub fn default_model(&self, model_name: &str) -> Result<ModelIden> {
		// -- First get the default ModelInfo
		let adapter_kind = AdapterKind::from_model(model_name)?;
		let model_iden = ModelIden::new(adapter_kind, model_name);
		Ok(model_iden)
	}

	/// Deprecated: use `Client::resolve_service_target`.
	#[deprecated(note = "use `client.resolve_service_target(model_name)`")]
	pub async fn resolve_model_iden(&self, model_name: &str) -> Result<ModelIden> {
		let model = self.default_model(model_name)?;
		let target = self.config().resolve_service_target(model).await?;
		Ok(target.model)
	}

	/// Resolves the service target (endpoint, auth, and model) for the given model name.
	pub async fn resolve_service_target(&self, model_name: &str) -> Result<ServiceTarget> {
		let model = self.default_model(model_name)?;
		self.config().resolve_service_target(model).await
	}

	/// Sends a chat request and returns the full response.
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
		let target = self.config().resolve_service_target(model).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		let WebRequestData {
			mut url,
			mut headers,
			payload,
		} = AdapterDispatcher::to_web_request_data(target, ServiceType::Chat, chat_req, options_set.clone())?;

		if let AuthData::RequestOverride {
			url: override_url,
			headers: override_headers,
		} = auth_data
		{
			url = override_url;
			headers = override_headers;
		};

		let web_res =
			self.web_client()
				.do_post(&url, &headers, payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model.clone(),
					webc_error,
				})?;

		let chat_res = AdapterDispatcher::to_chat_response(model, web_res, options_set)?;

		Ok(chat_res)
	}

	/// Streams a chat response.
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
		let target = self.config().resolve_service_target(model).await?;
		let model = target.model.clone();
		let auth_data = target.auth.clone();

		let WebRequestData {
			mut url,
			mut headers,
			payload,
		} = AdapterDispatcher::to_web_request_data(target, ServiceType::ChatStream, chat_req, options_set.clone())?;

		// TODO: Need to check this.
		//       This was part of the 429c5cee2241dbef9f33699b9c91202233c22816 commit
		//       But now it is missing in the the exec_chat(..) above, which is probably an issue.
		if let AuthData::RequestOverride {
			url: override_url,
			headers: override_headers,
		} = auth_data
		{
			url = override_url;
			headers = override_headers;
		};

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

	/// Creates embeddings for a single input string.
	pub async fn embed(
		&self,
		model: &str,
		input: impl Into<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new(input);
		self.exec_embed(model, embed_req, options).await
	}

	/// Creates embeddings for multiple input strings.
	pub async fn embed_batch(
		&self,
		model: &str,
		inputs: Vec<String>,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let embed_req = EmbedRequest::new_batch(inputs);
		self.exec_embed(model, embed_req, options).await
	}

	/// Sends an embedding request and returns the response.
	pub async fn exec_embed(
		&self,
		model: &str,
		embed_req: EmbedRequest,
		options: Option<&EmbedOptions>,
	) -> Result<EmbedResponse> {
		let options_set = EmbedOptionsSet::new()
			.with_request_options(options)
			.with_client_options(self.config().embed_options());

		let model = self.default_model(model)?;
		let target = self.config().resolve_service_target(model).await?;
		let model = target.model.clone();

		let WebRequestData { headers, payload, url } =
			AdapterDispatcher::to_embed_request_data(target, embed_req, options_set.clone())?;

		let web_res =
			self.web_client()
				.do_post(&url, &headers, payload)
				.await
				.map_err(|webc_error| Error::WebModelCall {
					model_iden: model.clone(),
					webc_error,
				})?;

		let res = AdapterDispatcher::to_embed_response(model, web_res, options_set)?;

		Ok(res)
	}
}
