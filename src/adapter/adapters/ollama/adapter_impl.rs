//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::adapters::support::get_api_key;
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::EmbedResponse;
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{embed, Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::{json, Value};
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
		// FIXME: This is hardcoded to the default endpoint; it should take the endpoint as an argument.
		let endpoint = Self::default_endpoint();
		let base_url = endpoint.base_url();
		let url = format!("{base_url}models");

		// TODO: Need to get the WebClient from the client.
		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &[]).await.map_err(|webc_error| Error::WebAdapterCall {
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

	fn get_service_url(model_iden: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> String {
		OpenAIAdapter::util_get_service_url(model_iden, service_type, endpoint)
	}

	/// We have not implemented embedding for Ollama yet
	fn get_embed_url(_model_iden: &ModelIden, endpoint: Endpoint) -> Option<String> {
		// Remove the OpenAI compatibility prefix to hop on main Ollama API
		let base_url = endpoint.base_url().replace("v1/", "");

		Some(format!("{base_url}api/embed"))
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, chat_options)
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

	fn embed(
		service_target: ServiceTarget,
		embed_req: crate::embed::SingleEmbedRequest,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = service_target;
		let model_name = &model.model_name;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_embed_url(&model, endpoint).ok_or(Error::EmbeddingNotSupported {
			model_iden: model.clone(),
		})?;

		// -- headers
		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let payload = json!({
			"model": model_name,
			"input": embed_req.document,
		});

		// Ollama does not support custom embedding dimensions
		// if let Some(dimensions) = options_set.dimensions() {
		// 	payload.x_insert("dimensions", dimensions)?;
		// }

		Ok(WebRequestData { url, headers, payload })
	}

	fn embed_batch(
		service_target: ServiceTarget,
		embed_req: crate::embed::BatchEmbedRequest,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { model, auth, endpoint } = service_target;
		let model_name = &model.model_name;

		// -- api_key
		let api_key = get_api_key(auth, &model)?;

		// -- url
		let url = Self::get_embed_url(&model, endpoint).ok_or(Error::EmbeddingNotSupported {
			model_iden: model.clone(),
		})?;

		// -- headers
		let headers = vec![
			// headers
			("Authorization".to_string(), format!("Bearer {api_key}")),
		];

		let payload = json!({
			"model": model_name,
			"input": embed_req.documents,
		});

		// Ollama does not support custom embedding dimensions
		// if let Some(dimensions) = options_set.dimensions() {
		// 	payload.x_insert("dimensions", dimensions)?;
		// }

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		let WebResponse { mut body, .. } = web_response;

		// Ollama does not return the usage
		let usage = embed::MetaUsage::default();

		// -- Capture the content
		let embeddings = body.x_take("embeddings").map(|embeddings: Vec<Vec<f64>>| {
			embeddings
				.into_iter()
				.filter_map(|embedding: Vec<f64>| Some(embed::EmbeddingObject { index: None, embedding }))
				.collect::<Vec<embed::EmbeddingObject>>()
		})?;

		Ok(EmbedResponse {
			embeddings,
			usage,
			model_iden,
		})
	}
}
