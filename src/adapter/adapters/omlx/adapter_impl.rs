use crate::ModelIden;
use crate::adapter::adapters::openai::{OpenAIAdapter, ToWebRequestDataOptions};
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStreamResponse};
use crate::embed::{EmbedOptionsSet, EmbedRequest, EmbedResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebClient, WebResponse};
use crate::{Result, ServiceTarget};
use reqwest::RequestBuilder;
use serde_json::json;
use value_ext::JsonValueExt;

pub struct OmlxAdapter;

impl Adapter for OmlxAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some("OMLX_API_KEY");

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		const DEFAULT_URL: &str = "http://127.0.0.1:8000/v1/";
		let url = std::env::var("OMLX_ENDPOINT").unwrap_or_else(|_| DEFAULT_URL.to_string());
		Endpoint::from_owned(url)
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		match Self::DEFAULT_API_KEY_ENV_NAME {
			Some(env_name) => {
				if std::env::var(env_name).is_ok() {
					AuthData::from_env(env_name)
				} else {
					AuthData::None
				}
			}
			None => AuthData::None,
		}
	}

	async fn all_model_names(
		kind: AdapterKind,
		endpoint: Endpoint,
		auth: AuthData,
		web_client: &WebClient,
	) -> Result<Vec<String>> {
		OpenAIAdapter::list_model_names_for_end_target(kind, endpoint, auth, web_client).await
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		OpenAIAdapter::util_get_service_url(model, service_type, endpoint)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		// for omlx, we allow no api keys
		let options = ToWebRequestDataOptions {
			allow_no_api_key: true,
			..Default::default()
		};

		let mut web_req_data =
			OpenAIAdapter::util_to_web_request_data(target, service_type, chat_req, options_set, Some(options))?;

		// -- Set the omlx specific reasoning block
		// NOTE: Not 100% sure this is honored by the omlx
		let reasoning_effort = web_req_data.payload.x_get_str("reasoning_effort").map(|r| r.to_string()).ok();

		if let Some(reasoning_effort) = reasoning_effort {
			web_req_data.payload["chat_template_kwargs"] = json!({
				"enable_thinking": true,
				"reasoning_effort": reasoning_effort,
				"preserve_thinking": false,
			});
		}

		Ok(web_req_data)
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
		service_target: ServiceTarget,
		embed_req: EmbedRequest,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		OpenAIAdapter::to_embed_request_data(service_target, embed_req, options_set)
	}

	fn to_embed_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		options_set: EmbedOptionsSet<'_, '_>,
	) -> Result<EmbedResponse> {
		OpenAIAdapter::to_embed_response(model_iden, web_response, options_set)
	}
}
