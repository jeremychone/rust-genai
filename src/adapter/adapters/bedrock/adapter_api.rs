//! Bedrock adapter backed by Bedrock's simple Bearer-token auth.
//!
//! Reads the token from `BEDROCK_API_KEY`, falling back to `AWS_BEARER_TOKEN_BEDROCK` — the name
//! AWS documents.
//!
//! See: https://docs.aws.amazon.com/bedrock/latest/userguide/api-keys.html

use crate::adapter::adapters::bedrock::converse::{build_converse_payload, parse_converse_response};
use crate::adapter::adapters::bedrock::shared::{
	BEDROCK_RUNTIME_HOST_PREFIX, DEFAULT_REGION, async_stream_bytes, build_service_url, region_from_env,
};
use crate::adapter::adapters::bedrock::streamer::BedrockStreamer;
use crate::adapter::adapters::support::get_api_key;
use crate::adapter::{Adapter, AdapterKind, ServiceType, WebRequestData};
use crate::chat::{ChatOptionsSet, ChatRequest, ChatResponse, ChatStream, ChatStreamResponse};
use crate::resolver::{AuthData, Endpoint};
use crate::webc::{WebClient, WebResponse};
use crate::{Error, Headers, ModelIden, Result, ServiceTarget};
use reqwest::RequestBuilder;

pub struct BedrockApiAdapter;

impl BedrockApiAdapter {
	pub const API_KEY_DEFAULT_ENV_NAME: &'static str = "BEDROCK_API_KEY";

	/// AWS's documented name for Bedrock bearer tokens; used as a fallback.
	pub const API_KEY_AWS_ENV_NAME: &'static str = "AWS_BEARER_TOKEN_BEDROCK";

	/// Candidate env vars, in priority order.
	fn api_key_env_names() -> [&'static str; 2] {
		[Self::API_KEY_DEFAULT_ENV_NAME, Self::API_KEY_AWS_ENV_NAME]
	}

	/// Resolves the token for this adapter's default env lookup: `BEDROCK_API_KEY` first, then
	/// `AWS_BEARER_TOKEN_BEDROCK`. Any other `AuthData` (explicit key, custom env name, resolver
	/// result) is used as-is.
	fn resolve_api_key(auth: AuthData, model: &ModelIden) -> Result<String> {
		let uses_adapter_default_env = matches!(
			&auth,
			AuthData::FromEnv(env_name) if env_name.as_str() == Self::API_KEY_DEFAULT_ENV_NAME
		);

		if !uses_adapter_default_env {
			return get_api_key(auth, model);
		}

		if let Some(api_key) = first_set_env_var(|name| std::env::var(name)) {
			return Ok(api_key);
		}

		Err(Error::Resolver {
			model_iden: model.clone(),
			resolver_error: crate::resolver::Error::ApiKeyEnvNotFound {
				env_name: Self::api_key_env_names().join(" or "),
			},
		})
	}

	fn resolve_region() -> String {
		region_from_env().unwrap_or_else(|| DEFAULT_REGION.to_string())
	}

	fn endpoint_for_region(region: &str) -> String {
		format!("https://{BEDROCK_RUNTIME_HOST_PREFIX}.{region}.amazonaws.com/")
	}
}

impl Adapter for BedrockApiAdapter {
	const DEFAULT_API_KEY_ENV_NAME: Option<&'static str> = Some(Self::API_KEY_DEFAULT_ENV_NAME);

	fn default_endpoint(_kind: AdapterKind) -> Endpoint {
		Endpoint::from_owned(Self::endpoint_for_region(&Self::resolve_region()))
	}

	fn default_auth(_kind: AdapterKind) -> AuthData {
		AuthData::from_env(Self::API_KEY_DEFAULT_ENV_NAME)
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
		build_service_url(model, service_type, endpoint, AdapterKind::BedrockApi)
	}

	fn to_web_request_data(
		target: ServiceTarget,
		service_type: ServiceType,
		chat_req: ChatRequest,
		options_set: ChatOptionsSet<'_, '_>,
	) -> Result<WebRequestData> {
		let ServiceTarget { endpoint, auth, model } = target;

		let api_key = Self::resolve_api_key(auth, &model)?;
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
		let frame_tap = bedrock_stream.frame_tap();
		let chat_stream = ChatStream::from_inter_stream(bedrock_stream).with_frame_tap(frame_tap);
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

/// Returns the value of the first candidate env var set to a non-empty value.
fn first_set_env_var(lookup: impl Fn(&str) -> std::result::Result<String, std::env::VarError>) -> Option<String> {
	BedrockApiAdapter::api_key_env_names()
		.into_iter()
		.find_map(|name| lookup(name).ok().filter(|value| !value.is_empty()))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn prefers_the_genai_env_var() {
		let key = first_set_env_var(|name| match name {
			"BEDROCK_API_KEY" | "AWS_BEARER_TOKEN_BEDROCK" => Ok(format!("{name}-value")),
			_ => Err(std::env::VarError::NotPresent),
		});
		assert_eq!(key.as_deref(), Some("BEDROCK_API_KEY-value"));
	}

	#[test]
	fn falls_back_to_the_aws_env_var() {
		let key = first_set_env_var(|name| match name {
			"AWS_BEARER_TOKEN_BEDROCK" => Ok("aws-token".to_string()),
			_ => Err(std::env::VarError::NotPresent),
		});
		assert_eq!(key.as_deref(), Some("aws-token"));
	}

	#[test]
	fn no_env_var_set_yields_none() {
		let key = first_set_env_var(|_| Err(std::env::VarError::NotPresent));
		assert_eq!(key, None);
	}

	#[test]
	fn empty_env_var_falls_through_to_the_next_candidate() {
		let key = first_set_env_var(|name| match name {
			"BEDROCK_API_KEY" => Ok(String::new()),
			"AWS_BEARER_TOKEN_BEDROCK" => Ok("aws-token".to_string()),
			_ => Err(std::env::VarError::NotPresent),
		});
		assert_eq!(key.as_deref(), Some("aws-token"));
	}
}
