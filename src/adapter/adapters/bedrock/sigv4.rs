//! SigV4 signing + credential resolution for Bedrock.
//!
//! Credentials are resolved once via `aws-config`'s default chain (env → profile → SSO → IMDS →
//! AssumeRole) and cached in a `OnceCell`. Signing itself happens per-request via `aws-sigv4`.

use crate::Headers;
use crate::{Error, Result};
use aws_credential_types::Credentials;
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use std::time::SystemTime;
use tokio::sync::OnceCell;

/// Service name for SigV4 scope; matches the service expected by `bedrock-runtime`.
pub(super) const BEDROCK_SERVICE: &str = "bedrock";

/// Cached credential provider + resolved region.
///
/// `aws-config` performs async IO for some providers (IMDS, SSO), so we resolve lazily.
static CREDS_CACHE: OnceCell<CachedCreds> = OnceCell::const_new();

#[derive(Clone)]
pub(super) struct CachedCreds {
	pub creds: Credentials,
	pub region: String,
}

/// Returns cached credentials + region, loading them on first use.
pub(super) async fn get_credentials() -> Result<CachedCreds> {
	let cached = CREDS_CACHE
		.get_or_try_init(load_credentials_uncached)
		.await?;
	Ok(cached.clone())
}

async fn load_credentials_uncached() -> Result<CachedCreds> {
	use aws_config::BehaviorVersion;
	use aws_credential_types::provider::ProvideCredentials;

	let config = aws_config::defaults(BehaviorVersion::latest()).load().await;

	let region = config
		.region()
		.map(|r| r.as_ref().to_string())
		.or_else(|| std::env::var("AWS_REGION").ok())
		.or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
		.unwrap_or_else(|| "us-east-1".to_string());

	let provider = config
		.credentials_provider()
		.ok_or_else(|| Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::BedrockSigv4,
			feature: "AWS credentials (no provider found in default chain)".to_string(),
		})?;

	let creds = provider
		.provide_credentials()
		.await
		.map_err(|err| Error::AdapterNotSupported {
			adapter_kind: crate::adapter::AdapterKind::BedrockSigv4,
			feature: format!("AWS credential resolution failed: {err}"),
		})?;

	Ok(CachedCreds { creds, region })
}

/// Extract the region from an already-loaded credentials snapshot.
/// Callers that need a request-time override should pass it to [`sign_request`] directly.
pub(super) fn cached_region(cached: &CachedCreds) -> &str {
	&cached.region
}

/// Sign a POST request (url + JSON body) for Bedrock Runtime and return the resulting
/// headers ready to be merged into [`Headers`]. The body is passed by reference so we don't
/// double-copy.
pub(super) fn sign_request(
	creds: &Credentials,
	region: &str,
	url: &str,
	body: &[u8],
) -> Result<Headers> {
	let identity = creds.clone().into();

	let signing_params = v4::SigningParams::builder()
		.identity(&identity)
		.region(region)
		.name(BEDROCK_SERVICE)
		.time(SystemTime::now())
		.settings(SigningSettings::default())
		.build()
		.map_err(|err| sign_err(format!("signing params: {err}")))?
		.into();

	// Minimum headers SigV4 needs to hash: Host + Content-Type. We pass the full body as a
	// `SignableBody::Bytes` so the signer computes x-amz-content-sha256 for us.
	let host = url_host(url).ok_or_else(|| sign_err(format!("could not extract host from url: {url}")))?;

	// The sig headers to include at signing time. We pass the ones we intend to send.
	let signing_headers: Vec<(&str, &str)> = vec![
		("host", host),
		("content-type", "application/json"),
	];

	let signable = SignableRequest::new(
		"POST",
		url,
		signing_headers.into_iter(),
		SignableBody::Bytes(body),
	)
	.map_err(|err| sign_err(format!("signable request: {err}")))?;

	let (signing_instructions, _sig) = sign(signable, &signing_params)
		.map_err(|err| sign_err(format!("sign: {err}")))?
		.into_parts();

	// SigningInstructions carries headers (and possibly query params) to attach to the
	// outgoing request.
	let mut genai_headers_vec: Vec<(String, String)> = vec![("content-type".to_string(), "application/json".to_string())];
	for (name, value) in signing_instructions.headers() {
		genai_headers_vec.push((name.to_string(), value.to_string()));
	}

	Ok(Headers::from(genai_headers_vec))
}

fn sign_err(msg: String) -> Error {
	Error::AdapterNotSupported {
		adapter_kind: crate::adapter::AdapterKind::BedrockSigv4,
		feature: format!("SigV4 signing failed: {msg}"),
	}
}

fn url_host(url: &str) -> Option<&str> {
	// Minimal host extraction: strip scheme, take up to first '/' or ':' or end.
	let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
	let end = without_scheme
		.find(|c: char| c == '/' || c == ':' || c == '?')
		.unwrap_or(without_scheme.len());
	let host = &without_scheme[..end];
	if host.is_empty() { None } else { Some(host) }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extracts_host_from_url() {
		assert_eq!(
			url_host("https://bedrock-runtime.us-east-1.amazonaws.com/model/foo/converse"),
			Some("bedrock-runtime.us-east-1.amazonaws.com")
		);
		assert_eq!(
			url_host("https://bedrock-runtime.us-east-1.amazonaws.com"),
			Some("bedrock-runtime.us-east-1.amazonaws.com")
		);
		assert_eq!(url_host("http://localhost:4566/model/x/converse"), Some("localhost"));
	}
}
