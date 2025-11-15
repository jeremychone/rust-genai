//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Result};
use crate::{ModelIden, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::Value;
use value_ext::JsonValueExt;

pub struct OllamaAdapter;

/// Note: For now, it uses the OpenAI compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base Ollama API supports `application/x-ndjson` for streaming, whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "http://localhost:11434/v1/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_single("ollama")
	}

	/// Note 1: For now, this adapter is the only one making a full request to the Ollama server
	/// Note 2: Use the OpenAI API to communicate with the Ollama server (https://platform.openai.com/docs/api-reference/models/list)
	///
	/// TODO: This will use the default endpoint.
	///       Later, we might add another function with an endpoint, so the user can provide a custom endpoint.
	async fn all_model_names(adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let web_client = crate::webc::WebClient::default();
		let target = ServiceTarget {
			endpoint: Self::default_endpoint(),
			auth: Self::default_auth(),
			model: ModelIden::new(adapter_kind, "__ollama_list_models__"),
		};

		Self::all_model_names_with_service_target(adapter_kind, target, &web_client).await
	}

	fn get_service_url(model_iden: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::util_get_service_url(model_iden, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options, None)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		service_target: crate::ServiceTarget,
		embed_req: crate::embed::EmbedRequest,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::adapter::WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: crate::ModelIden,
		web_response: crate::webc::WebResponse,
		options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}

impl OllamaAdapter {
	pub(crate) async fn all_model_names_with_service_target(
		adapter_kind: AdapterKind,
		target: ServiceTarget,
		web_client: &crate::webc::WebClient,
	) -> Result<Vec<String>> {
		let ServiceTarget { endpoint, auth, model } = target;

		let mut url = format!("{}models", endpoint.base_url());
		let mut headers: Vec<(String, String)> = Vec::new();

		match auth {
			AuthData::RequestOverride {
				url: override_url,
				headers: override_headers,
			} => {
				url = override_url;
				headers.extend(override_headers.into_iter());
			}
			auth => {
				let api_key = get_api_key(auth, &model)?;
				if !api_key.is_empty() {
					headers.push(("Authorization".to_string(), format!("Bearer {api_key}")));
				}
			}
		}

		let mut res = web_client
			.do_get(&url, &headers)
			.await
			.map_err(|webc_error| Error::WebAdapterCall {
				adapter_kind,
				webc_error,
			})?;

		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("data")? {
			for mut model in models_value {
				let model_name: String = model.x_take("id")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}
}
