//! Bedrock adapter backed by Bedrock's simple Bearer-token auth (`BEDROCK_API_KEY`).
//!
//! Requires the `bedrock-api` Cargo feature. Pulls in no new dependencies.
//!
//! See: https://docs.aws.amazon.com/bedrock/latest/userguide/api-keys.html

use crate::adapter::adapters::bedrock::converse::{build_converse_payload, parse_converse_response};
use crate::adapter::adapters::bedrock::shared::{
	async_stream_bytes, build_service_url, BEDROCK_RUNTIME_HOST_PREFIX,
};
use crate::adapter::adapters::bedrock::streamer::BedrockStreamer;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct BedrockApiAdapter;

impl BedrockApiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &'static str = "BEDROCK_API_KEY";

	fn resolve_region() -> String {
		std::env::var("AWS_REGION")
			.or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
			.unwrap_or_else(|_| "us-east-1".to_string())
	}

	fn endpoint_for_region(region: &str) -> String {
		format!("https://{BEDROCK_RUNTIME_HOST_PREFIX}.{region}.amazonaws.com/")
	}
}

impl Adapter for BedrockApiAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint() -> Endpoint {
		Endpoint::from_owned(Self::endpoint_for_region(&Self::resolve_region()))
	}

	fn default_auth() -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
	}

	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		Ok(crate::adapter::adapters::bedrock::shared::curated_model_names())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		build_service_url(model, service_type, endpoint, AdapterKind::BedrockApi)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;

		let api_key = get_api_key(auth, &model)?;
		let payload = build_converse_payload(&model, chat_req, options_set)?;
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		let headers = Headers::from(vec![
			("authorization".to_string(), format!("Bearer {api_key}")),
			("content-type".to_string(), "application/json".to_string()),
		]);

		Ok(WebRequestData { url, headers, payload })
	}

	fn to_chat_response(
		model_iden: ModelIden,
		web_response: WebResponse,
		_options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatResponse> {
		parse_converse_response(model_iden, web_response)
	}

	fn to_chat_stream(
		model_iden: ModelIden,
		reqwest_builder: RequestBuilder,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<ChatStreamResponse> {
		let stream = async_stream_bytes(reqwest_builder);
		let bedrock_stream = BedrockStreamer::new(Box::pin(stream), model_iden.clone(), options_set);
		let chat_stream = ChatStream::from_inter_stream(bedrock_stream);
		Ok(ChatStreamResponse {
			model_iden,
			stream: chat_stream,
		})
	}

	fn to_embed_request_data(
		_service_target: ServiceTarget,
		_embed_req: crate::embed::EmbedRequest,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::BedrockApi,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::BedrockApi,
			feature: "embeddings".to_string(),
		})
	}
}
