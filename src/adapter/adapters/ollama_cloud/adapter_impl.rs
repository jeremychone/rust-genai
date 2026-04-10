use crate::Headers;
use crate::adapter::adapters::ollama::OllamaAdapter;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;
use value_ext::JsonValueExt;

pub struct OllamaCloudAdapter;

impl OllamaCloudAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &str = "OLLAMA_API_KEY";
}

impl Adapter for OllamaCloudAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		const BASE_URL: &str = "https://ollama.com/";
		Endpoint::from_static(BASE_URL)
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	async fn all_model_names(adapter_kind: AdapterKind, endpoint: Endpoint, auth: AuthData) -> Result<Vec<String>> {
		let base_url = endpoint.base_url();
		let url = format!("{base_url}api/tags");

		let api_key = get_api_key(auth, &ModelIden::new(adapter_kind, ""))?;
		let headers = Headers::from(vec![("Authorization", format!("Bearer {api_key}"))]);

		let web_c = crate::webc::WebClient::default();
		let mut res = web_c.do_get(&url, &headers).await.map_err(|webc_error| Error::WebAdapterCall {
			adapter_kind,
			webc_error,
		})?;

		let mut models: Vec<String> = Vec::new();
		if let serde_json::Value::Array(models_value) = res.body.x_take("models")? {
			for mut model in models_value {
				let model_name: String = model.x_take("name")?;
				models.push(model_name);
			}
		} else {
			// TODO: Need to add tracing
		}
		Ok(models)
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OllamaAdapter::get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		chat_options: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let api_key = get_api_key(target.auth.clone(), &target.model)?;
		let mut web_req = OllamaAdapter::to_web_request_data(target, service_type, chat_req, chat_options)?;
		web_req
			.headers
			.merge(Headers::from(("Authorization", format!("Bearer {api_key}"))));
		Ok(web_req)
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		OllamaAdapter::to_chat_response(model_iden, web_response, options_set)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		OllamaAdapter::to_chat_stream(model_iden, reqwest_builder, options_set)
	}

	fn to_embed_request_data(
		service_target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let api_key = get_api_key(service_target.auth.clone(), &service_target.model)?;
		let mut web_req = OllamaAdapter::to_embed_request_data(service_target, embed_req, options_set)?;
		web_req
			.headers
			.merge(Headers::from(("Authorization", format!("Bearer {api_key}"))));
		Ok(web_req)
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		OllamaAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}
