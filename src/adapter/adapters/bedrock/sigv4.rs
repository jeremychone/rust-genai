//! SigV4 signing + credential resolution for Bedrock.
//!
//! Credentials come from `aws-config`'s default chain (env → profile → SSO → IMDS → AssumeRole)
//! and are cached until shortly before they expire; signing is per-request via `aws-sigv4`.
//!
//! The cache mirrors the AWS SDK's identity cache: expire `REFRESH_BUFFER` early, fall back to
//! `DEFAULT_EXPIRATION` when no expiry is reported, deduplicate concurrent refreshes, and jitter
//! to avoid lockstep. Without it, a long-lived process would keep signing with credentials that
//! expired an hour after start-up (`provide_credentials()` returns a frozen snapshot).

use crate::Headers;
use crate::{Error, Result};
use aws_credential_types::Credentials;
use aws_credential_types::provider::{ProvideCredentials, SharedCredentialsProvider};
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, OnceCell};

/// Service name for SigV4 scope; matches the service expected by `bedrock-runtime`.
pub(super) const BEDROCK_SERVICE: &str = "bedrock";

/// Treat credentials as expired this long before their real expiration.
/// Mirrors `DEFAULT_BUFFER_TIME` in the AWS SDK's identity cache.
const REFRESH_BUFFER: Duration = Duration::from_secs(10);

/// TTL when the provider reports no expiration (e.g. static env credentials).
/// Mirrors `DEFAULT_EXPIRATION` in the AWS SDK's identity cache.
const DEFAULT_EXPIRATION: Duration = Duration::from_secs(15 * 60);

/// Credential provider + region, resolved once per process.
static AWS_CONFIG: OnceCell<AwsConfig> = OnceCell::const_new();

/// Expiry-aware credentials cache; `None` until the first resolution.
static CREDS_CACHE: Mutex<Option<CachedCreds>> = Mutex::const_new(None);

struct AwsConfig {
	provider: SharedCredentialsProvider,
	region: String,
}

/// Resolved credentials and the region they were resolved for.
#[derive(Clone)]
pub(super) struct CachedCreds {
	pub creds: Credentials,
	pub region: String,
	/// Expiration (or default TTL) plus jitter; cache bookkeeping only.
	deadline: SystemTime,
}

/// Returns credentials + region, refreshing them when they are close to expiring.
///
/// Holding the lock across the refresh deduplicates concurrent callers, as the AWS SDK's
/// `ExpiringCache::get_or_load` does.
pub(super) async fn get_credentials() -> Result<CachedCreds> {
	let config = get_aws_config().await?;
	let now = SystemTime::now();

	let mut cache = CREDS_CACHE.lock().await;

	// Fast path: the cached credentials are still fresh.
	if let Some(cached) = cache.as_ref()
		&& !expired(cached.deadline, now)
	{
		return Ok(cached.clone());
	}

	// Slow path: refresh.
	let creds = fetch_credentials(config).await?;
	let cached = CachedCreds {
		deadline: deadline(creds.expiry(), now, jitter_fraction()),
		creds,
		region: config.region.clone(),
	};
	cache.replace(cached.clone());

	Ok(cached)
}

async fn get_aws_config() -> Result<&'static AwsConfig> {
	AWS_CONFIG.get_or_try_init(load_aws_config).await
}

async fn load_aws_config() -> Result<AwsConfig> {
	use aws_config::BehaviorVersion;

	let config = aws_config::defaults(BehaviorVersion::latest()).load().await;

	let region = config
		.region()
		.map(|r| r.as_ref().to_string())
		.or_else(|| std::env::var("AWS_REGION").ok())
		.or_else(|| std::env::var("AWS_DEFAULT_REGION").ok())
		.unwrap_or_else(|| "us-east-1".to_string());

	let provider = config
		.credentials_provider()
		.ok_or_else(|| aws_err("AWS credentials (no provider found in default chain)"))?;

	Ok(AwsConfig { provider, region })
}

async fn fetch_credentials(config: &AwsConfig) -> Result<Credentials> {
	config
		.provider
		.provide_credentials()
		.await
		.map_err(|err| aws_err(format!("AWS credential resolution failed: {err}")))
}

/// `true` once `now` is within `REFRESH_BUFFER` of the deadline.
/// Mirrors the AWS SDK's expiration check.
fn expired(deadline: SystemTime, now: SystemTime) -> bool {
	deadline.checked_sub(REFRESH_BUFFER).is_none_or(|refresh_at| now >= refresh_at)
}

/// Provider expiration, or a default TTL, plus jitter.
/// Mirrors how the AWS SDK stores `expiration + jitter`.
fn deadline(expiry: Option<SystemTime>, now: SystemTime, jitter_fraction: f64) -> SystemTime {
	let base = expiry.unwrap_or(now + DEFAULT_EXPIRATION);
	base + REFRESH_BUFFER.mul_f64(jitter_fraction)
}

/// A random fraction in `[0.0, 0.5)`, the range the AWS SDK multiplies into its buffer.
fn jitter_fraction() -> f64 {
	// `RandomState` is seeded from OS entropy, so no `rand` dependency is needed.
	let mut hasher = RandomState::new().build_hasher();
	hasher.write_u64(0);
	// Top 53 bits give a uniform value in [0, 1).
	((hasher.finish() >> 11) as f64 / (1_u64 << 53) as f64) * 0.5
}

/// Extract the region from an already-loaded credentials snapshot.
/// Callers that need a request-time override should pass it to [`sign_request`] directly.
pub(super) fn cached_region(cached: &CachedCreds) -> &str {
	&cached.region
}

/// Sign a POST request (url + JSON body) for Bedrock Runtime and return the resulting
/// headers ready to be merged into [`Headers`]. The body is passed by reference so we don't
/// double-copy.
pub(super) fn sign_request(creds: &Credentials, region: &str, url: &str, body: &[u8]) -> Result<Headers> {
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
	let signing_headers: Vec<(&str, &str)> = vec![("host", host), ("content-type", "application/json")];

	let signable = SignableRequest::new("POST", url, signing_headers.into_iter(), SignableBody::Bytes(body))
		.map_err(|err| sign_err(format!("signable request: {err}")))?;

	let (signing_instructions, _sig) = sign(signable, &signing_params)
		.map_err(|err| sign_err(format!("sign: {err}")))?
		.into_parts();

	// SigningInstructions carries headers (and possibly query params) to attach to the
	// outgoing request.
	let mut genai_headers_vec: Vec<(String, String)> =
		vec![("content-type".to_string(), "application/json".to_string())];
	for (name, value) in signing_instructions.headers() {
		genai_headers_vec.push((name.to_string(), value.to_string()));
	}

	Ok(Headers::from(genai_headers_vec))
}

fn aws_err(feature: impl Into<String>) -> Error {
	Error::AdapterNotSupported {
		adapter_kind: crate::adapter::AdapterKind::BedrockSigv4,
		feature: feature.into(),
	}
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

	fn epoch_secs(secs: u64) -> SystemTime {
		SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
	}

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

	#[test]
	fn expired_only_within_the_buffer() {
		let now = epoch_secs(1_000);

		// Still outside the buffer: reuse the cached credentials.
		assert!(!expired(epoch_secs(1_060), now));
		// Inside the buffer: refresh before they actually expire.
		assert!(expired(epoch_secs(1_010), now));
		// Exactly at the buffer boundary.
		assert!(expired(epoch_secs(1_000), now));
		// Already past the real expiration.
		assert!(expired(epoch_secs(999), now));
	}

	#[test]
	fn deadline_uses_default_expiration_when_provider_reports_none() {
		let now = epoch_secs(1_000);
		// Static env credentials have no expiry, so they still get a TTL.
		assert_eq!(deadline(None, now, 0.0), now + DEFAULT_EXPIRATION);
	}

	#[test]
	fn deadline_uses_provider_expiration_when_present() {
		let now = epoch_secs(1_000);
		let expiry = epoch_secs(4_600);
		assert_eq!(deadline(Some(expiry), now, 0.0), expiry);
	}

	#[test]
	fn deadline_jitter_never_exceeds_half_the_buffer() {
		let now = epoch_secs(1_000);
		let expiry = epoch_secs(4_600);
		let upper = expiry + REFRESH_BUFFER.div_f64(2.0);

		for _ in 0..1_000 {
			let with_jitter = deadline(Some(expiry), now, jitter_fraction());
			assert!(with_jitter >= expiry, "jitter must not shorten the deadline");
			assert!(with_jitter < upper, "jitter must stay below half the buffer");
		}
	}

	#[test]
	fn jitter_fraction_is_in_the_aws_sdk_range() {
		for _ in 0..1_000 {
			let fraction = jitter_fraction();
			assert!(
				(0.0..0.5).contains(&fraction),
				"jitter fraction out of range: {fraction}"
			);
		}
	}
}
