//! API DOC: https://github.com/ollama/ollama/blob/main/docs/openai.md

use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::{Adapter, AdapterConfig, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatRequest, ChatOptionsSet, ChatResponse, ChatStreamResponse};
use crate::support::value_ext::ValueExt;
use crate::webc::WebResponse;
use crate::{ConfigSet, ModelInfo};
use crate::{Error, Result};
use reqwest::RequestBuilder;
use serde_json::Value;
use std::sync::OnceLock;

pub struct OllamaAdapter;

// The OpenAI Compatibility base URL
const BASE_URL: &str = "http://localhost:11434/v1/";
const OLLAMA_BASE_URL: &str = "http://localhost:11434/api/";

/// Note: For now, it uses the openai compatibility layer
///       (https://github.com/ollama/ollama/blob/main/docs/openai.md)
///       Since the base ollama API supports `application/x-ndjson` for streaming whereas others support `text/event-stream`
impl Adapter for OllamaAdapter {
	/// Note: For now returns empty as it should probably do a request to the ollama server
	async fn all_model_names(adapter_kind: AdapterKind) -> Result<Vec<String>> {
		let url = format!("{OLLAMA_BASE_URL}tags");

		// TODO: need to get the WebClient from the client.
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
			// TODO: need to add tracing
			// error!("OllamaAdapter::list_models did not have any models {res:?}");
		}

		Ok(models)
	}

	fn default_adapter_config(_kind: AdapterKind) -> &'static AdapterConfig {
		static INSTANCE: OnceLock<AdapterConfig> = OnceLock::new();
		INSTANCE.get_or_init(AdapterConfig::default)
	}

	fn get_service_url(model_info: ModelInfo, service_type: ServiceType) -> String {
		OpenAIAdapter::util_get_service_url(model_info, service_type, BASE_URL)
	}

	fn to_web_request_data(
		model_info: ModelInfo,
		_config_set: &ConfigSet<'_>,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let url = Self::get_service_url(model_info.clone(), service_type);

		OpenAIAdapter::util_to_web_request_data(model_info, url, chat_req, service_type, options_set, "ollama", true)
	}

	fn to_chat_response(model_info: ModelInfo, web_response: WebResponse) -> Result<ChatResponse> {
		OpenAIAdapter::to_chat_response(model_info, web_response)
	}

	fn to_chat_stream(
		model_info: ModelInfo,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OpenAIAdapter::to_chat_stream(model_info, reqwest_builder, options_set)
	}
}
