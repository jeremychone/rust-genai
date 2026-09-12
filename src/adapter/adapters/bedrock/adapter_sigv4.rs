//! Bedrock adapter backed by SigV4 request signing + the full AWS credential chain.
//!
//! Requires the `bedrock-sigv4` Cargo feature.

use crate::adapter::adapters::bedrock::converse::{build_converse_payload, parse_converse_response};
use crate::adapter::adapters::bedrock::shared::{
	BEDROCK_RUNTIME_HOST_PREFIX, DEFAULT_REGION, async_stream_bytes, build_service_url, region_from_env,
};
use crate::adapter::adapters::bedrock::sigv4::{get_credentials, profile_from_auth, sign_request};
use crate::adapter::adapters::bedrock::streamer::BedrockStreamer;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebClient, WebResponse};
use crate::{Error, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct BedrockSigv4Adapter;

impl BedrockSigv4Adapter {
	fn resolve_region() -> String {
		region_from_env().unwrap_or_else(|| DEFAULT_REGION.to_string())
	}

	pub(super) fn endpoint_for_region(region: &str) -> String {
		format!("https://{BEDROCK_RUNTIME_HOST_PREFIX}.{region}.amazonaws.com/")
	}
}

impl Adapter for BedrockSigv4Adapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = None;

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		let region = Self::resolve_region();
		Endpoint::from_owned(Self::endpoint_for_region(&region))
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		// Credentials come from the AWS chain at request time; a profile can be selected per client
		// via `ProviderConfig`/`AuthData`, which reaches us as `ServiceTarget::auth`.
		AuthData::None
	}

	async fn all_model_names(
		_kind: AdapterKind,
		_endpoint: Endpoint,
		_auth: AuthData,
		_web_client: &WebClient,
	) -> Result<Vec<String>> {
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
		let ServiceTarget { endpoint, auth, model } = target;

		// 1. Resolve the selected AWS profile (if any) and its credentials, refreshing if needed.
		let profile = profile_from_auth(&auth)?;
		let cached = tokio_block_on(get_credentials(profile.as_deref()))?;

		// 2. Align the default endpoint with the region we sign for.
		let endpoint = override_endpoint_region(endpoint, &cached.region);

		// 3. Build the Converse JSON payload.
		let payload = build_converse_payload(&model, chat_req, options_set)?;

		// 4. Compute URL
		let url = Self::get_service_url(&model, service_type, endpoint)?;

		// 5. Sign the request — we serialize the body for the payload hash.
		let body_bytes = serde_json::to_vec(&payload)?;
		let headers = sign_request(&cached.creds, &cached.region, &url, &body_bytes)?;

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

/// Rebuilds the default endpoint for the region we sign for (which may come from
/// `~/.aws/config`); a user-supplied endpoint is left alone.
fn override_endpoint_region(endpoint: Endpoint, signed_region: &str) -> Endpoint {
	let env_default = BedrockSigv4Adapter::endpoint_for_region(&BedrockSigv4Adapter::resolve_region());
	if endpoint.base_url() == env_default {
		Endpoint::from_owned(BedrockSigv4Adapter::endpoint_for_region(signed_region))
	} else {
		endpoint
	}
}

/// Run a future to completion on the current Tokio runtime; the adapter trait is sync, so this
/// blocks the calling worker thread.
fn tokio_block_on<F: std::future::Future>(fut: F) -> F::Output {
	tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A region that differs from the ambient `AWS_REGION`, to keep the test deterministic.
	fn region_different_from_env() -> &'static str {
		if BedrockSigv4Adapter::resolve_region() == "eu-west-1" {
			"us-west-2"
		} else {
			"eu-west-1"
		}
	}

	/// The URL must target the region we sign for, else the signature carries one region while
	/// the Host header points at another (`SignatureDoesNotMatch`).
	#[test]
	fn request_url_targets_the_region_we_sign_for() {
		let signing_region = region_different_from_env();
		let model = ModelIden::new(
			AdapterKind::BedrockSigv4,
			"us.anthropic.claude-sonnet-4-5-20250929-v1:0",
		);

		let endpoint = BedrockSigv4Adapter::default_endpoint(AdapterKind::BedrockSigv4);
		let endpoint = override_endpoint_region(endpoint, signing_region);
		let url = BedrockSigv4Adapter::get_service_url(&model, ServiceType::Chat, endpoint)
			.expect("the Bedrock Converse URL should build");

		assert!(
			url.contains(&format!("bedrock-runtime.{signing_region}.amazonaws.com")),
			"request URL region != signing region; SigV4 would fail with SignatureDoesNotMatch. \
			 signing_region={signing_region} url={url}"
		);
	}

	/// A user-supplied endpoint must not be rewritten.
	#[test]
	fn user_supplied_endpoint_is_left_alone() {
		let custom = Endpoint::from_static("https://vpce-0123.bedrock-runtime.eu-west-1.vpce.amazonaws.com/");
		let endpoint = override_endpoint_region(custom, "us-east-1");

		assert_eq!(
			endpoint.base_url(),
			"https://vpce-0123.bedrock-runtime.eu-west-1.vpce.amazonaws.com/"
		);
	}
}
