//! Bedrock adapter backed by SigV4 request signing + the full AWS credential chain.
//!
//! Requires the `bedrock-sigv4` Cargo feature.

use crate::adapter::adapters::bedrock::converse::{build_converse_payload, parse_converse_response};
use crate::adapter::adapters::bedrock::shared::{
	async_stream_bytes, build_service_url, BEDROCK_RUNTIME_HOST_PREFIX,
};
use crate::adapter::adapters::bedrock::sigv4::{cached_region, get_credentials, sign_request};
use crate::adapter::adapters::bedrock::streamer::BedrockStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::WebResponse;
use crate::{Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct BedrockSigv4Adapter;

impl BedrockSigv4Adapter {
	fn resolve_region() -> String {
		std::env::var("AWS_REGION")
			.or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
			.unwrap_or_else(|_| "us-east-1".to_string())
	}

	pub(super) fn endpoint_for_region(region: &str) -> String {
		format!("https://{BEDROCK_RUNTIME_HOST_PREFIX}.{region}.amazonaws.com/")
	}
}

impl Adapter for BedrockSigv4Adapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = None;

	fn default_endpoint() -> Endpoint {
		let region = Self::resolve_region();
		Endpoint::from_owned(Self::endpoint_for_region(&region))
	}

	fn default_auth() -> AuthData {
		// Credentials come from the AWS default chain at request time.
		AuthData::None
	}

	async fn all_model_names(_kind: AdapterKind, _endpoint: Endpoint, _auth: AuthData) -> Result<Vec<String>> {
		Ok(crate::adapter::adapters::bedrock::shared::curated_model_names())
	}

	fn get_service_url(model: &ModelIden, service_type: ServiceType, endpoint: Endpoint) -> Result<String> {
		build_service_url(model, service_type, endpoint, AdapterKind::BedrockSigv4)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth: _, model } = target;

		// 1. Resolve credentials (cached). Only truly async on the first call per process.
		let cached = tokio_block_on(get_credentials())?;

		// 2. Determine region. Respect custom endpoint from ServiceTargetResolver.
		let endpoint = override_endpoint_region(endpoint, cached_region(&cached));

		// 3. Build the Converse JSON payload.
		let payload = build_converse_payload(&model, chat_req, options_set)?;

		// 4. Compute URL
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// 5. Sign the request — we serialize the body for the payload hash.
		let body_bytes = serde_json::to_vec(&payload)?;
		let headers = sign_request(&cached.creds, cached_region(&cached), &url, &body_bytes)?;

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
			adapter_kind: AdapterKind::BedrockSigv4,
			feature: "embeddings".to_string(),
		})
	}

	fn to_embed_response(
		_model_iden: ModelIden,
		_web_response: WebResponse,
		_options_set: crate::embed::EmbedOptionsSet<'_, '_>,
	) -> Result<crate::embed::EmbedResponse> {
		Err(Error::AdapterNotSupported {
			adapter_kind: AdapterKind::BedrockSigv4,
			feature: "embeddings".to_string(),
		})
	}
}

/// If the Endpoint's base URL is the default template with a placeholder region, substitute the
/// cached region. If the user supplied a custom endpoint, leave it alone.
fn override_endpoint_region(endpoint: Endpoint, cached_region: &str) -> Endpoint {
	let base = endpoint.base_url();
	if base.contains("bedrock-runtime..amazonaws.com") || base == "https://bedrock-runtime..amazonaws.com/" {
		Endpoint::from_owned(BedrockSigv4Adapter::endpoint_for_region(cached_region))
	} else {
		endpoint
	}
}

/// Synchronously run a future on the current Tokio runtime. The adapter trait is sync; the
/// credential cache only blocks on the very first call per process.
fn tokio_block_on<F: std::future::Future>(fut: F) -> F::Output {
	tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}
