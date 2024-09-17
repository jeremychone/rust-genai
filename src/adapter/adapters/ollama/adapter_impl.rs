//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::webc::WebResponse;
use crate::{ClientConfig, ModelIden};
use crate::{Error, Result};
use reqwest::RequestBuilder;
use serde_json::Value;
use value_ext::JsonValueExt;

pub struct OllamaAdapter;

// The OpenAI Compatibility base URL
const BASE_URL: &str = "http://localhost:11434/v1/";
const OLLAMA_BASE_URL: &str = "http://localhost:11434/api/";

/// Note: For now, it uses the OpenAI compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base Ollama API supports `application/x-ndjson` for streaming, whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	fn default_key_env_name(_kind: AdapterKind) -> Option<&'static str> {
		None
	}

	/// Note: For now, it returns empty as it should probably make a request to the Ollama server
	async fn all_model_names(adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let url = format!("{OLLAMA_BASE_URL}tags");

		// TODO: Need to get the WebClient from the client.
		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &[]).await.map_err(|webc_error| Error::WebAdapterCall {
			adapter_kind,
			webc_error,
		})?;

		let mut models: Vec<String> = Vec::new();

		if let Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("model")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}

	fn get_service_url(model_iden: ModelIden, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(model_iden, service_type, BASE_URL)
	}

	fn to_web_request_data(
		model_iden: ModelIden,
		client_config: &ClientConfig,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let url = Self::get_service_url(model_iden.clone(), service_type);

		OpenAIAdapter::util_to_web_request_data(model_iden, client_config, chat_req, service_type, options_set, url)
	}

	fn to_chat_response(model_iden: ModelIden, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_iden, web_response)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}
}
