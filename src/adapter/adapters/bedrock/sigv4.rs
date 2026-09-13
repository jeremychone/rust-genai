//! SigV4 signing + credential resolution for Bedrock.
//!
//! Credentials come from `aws-config`'s default chain (env → profile → SSO → IMDS → AssumeRole) and
//! are cached per AWS profile until shortly before they expire; signing is per-request via
//! `aws-sigv4`. The profile is selected per client as `AuthData::Key("<profile>")` (see
//! [`profile_from_auth`]); `AuthData::None` follows `AWS_PROFILE`, else `default`.
//!
//! The cache mirrors the AWS SDK's identity cache: expire `REFRESH_BUFFER` early, fall back to
//! `DEFAULT_EXPIRATION` when no expiry is reported, deduplicate concurrent refreshes, and jitter
//! to avoid lockstep. Without it, a long-lived process would keep signing with credentials that
//! expired an hour after start-up (`provide_credentials()` returns a frozen snapshot).

use super::shared::{DEFAULT_REGION, region_from_env};
use crate::Headers;
use crate::resolver::AuthData;
use crate::{Error, Result};
use aws_credential_types::Credentials;
use aws_credential_types::provider::{ProvideCredentials, SharedCredentialsProvider};
use aws_sigv4::http_request::{SignableBody, SignableRequest, SigningSettings, sign};
use aws_sigv4::sign::v4;
use std::collections::HashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, SystemTime};

/// Service name for SigV4 scope; matches the service expected by `bedrock-runtime`.
pub(super) const BEDROCK_SERVICE: &str = "bedrock";

/// `AuthData::MultiKeys` entry naming the AWS profile.
const PROFILE_AUTH_KEY: &str = "profile";

/// Treat credentials as expired this long before their real expiration.
/// Mirrors `DEFAULT_BUFFER_TIME` in the AWS SDK's identity cache.
const REFRESH_BUFFER: Duration = Duration::from_secs(10);

/// TTL when the provider reports no expiration (e.g. static env credentials).
/// Mirrors `DEFAULT_EXPIRATION` in the AWS SDK's identity cache.
const DEFAULT_EXPIRATION: Duration = Duration::from_secs(15 * 60);

/// AWS profile selector: a named profile, or `None` for the ambient chain.
type ProfileKey = Option<String>;

/// Credentials cached per AWS profile, refreshed in place when they expire.
///
/// The cache is keyed by profile name alone, so every client that selects one profile also shares
/// its region (the profile's `region` setting, else `AWS_REGION`).
static CREDS_CACHE: LazyLock<Mutex<HashMap<ProfileKey, CachedCreds>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Credentials for one AWS profile: what to sign with, the region they are valid for, and the
/// provider + deadline that keep them fresh.
#[derive(Clone)]
pub(super) struct CachedCreds {
	pub(super) creds: Credentials,
	pub(super) region: String,
	/// Refreshes these credentials; cache plumbing, not for callers.
	provider: SharedCredentialsProvider,
	/// Expiration (or default TTL) plus jitter; cache bookkeeping only.
	deadline: SystemTime,
}

/// Returns `profile`'s credentials and region, refreshing them when they are close to expiring.
///
/// `profile` is an AWS named profile; `None` follows the ambient chain (`AWS_PROFILE`, else
/// `default`). A failed refresh keeps the previous entry, so the next call retries.
pub(super) async fn get_credentials(profile: Option<&str>) -> Result<CachedCreds> {
	let key = normalize_profile(profile);
	let now = SystemTime::now();

	// Fast path: a fresh entry is served without waiting for any fetch in flight.
	if let Some(cached) = cached_for(&key)
		&& !expired(cached.deadline, now)
	{
		return Ok(cached);
	}

	// Slow path, resolved outside the lock: reuse this profile's provider if one is cached, else
	// load its config. Re-fetching from that provider is what refreshes temporary credentials
	// (SSO, AssumeRole, `credential_process`, IMDS).
	let (provider, region) = match cached_for(&key) {
		Some(cached) => (cached.provider.clone(), cached.region.clone()),
		None => load_aws_config(key.as_deref()).await?,
	};

	let creds = provider
		.provide_credentials()
		.await
		.map_err(|err| creds_err(key.as_deref(), err))?;

	let cached = CachedCreds {
		deadline: deadline(creds.expiry(), now, jitter_fraction()),
		creds,
		region,
		provider,
	};
	store(key, cached.clone());

	Ok(cached)
}

/// The cached credentials for a profile, if any.
fn cached_for(key: &ProfileKey) -> Option<CachedCreds> {
	CREDS_CACHE.lock().unwrap_or_else(|err| err.into_inner()).get(key).cloned()
}

/// Caches a profile's credentials.
fn store(key: ProfileKey, cached: CachedCreds) {
	CREDS_CACHE.lock().unwrap_or_else(|err| err.into_inner()).insert(key, cached);
}

/// Normalizes a profile selector: blank means the ambient chain, like an empty `AWS_PROFILE`.
fn normalize_profile(profile: Option<&str>) -> ProfileKey {
	profile.map(str::trim).filter(|profile| !profile.is_empty()).map(str::to_owned)
}

/// Extracts the AWS profile a caller selected, from the auth data resolved into `ServiceTarget`:
///
/// - `AuthData::None` → ambient chain (`AWS_PROFILE`, else `default`)
/// - `AuthData::Key("dev")` → profile `dev`
/// - `AuthData::FromEnv("MY_PROFILE")` → the profile named by that environment variable
/// - `AuthData::MultiKeys({ "profile": "dev" })` → profile `dev`, and a missing `profile` entry is
///   an error, since `MultiKeys` is the explicit form
///
/// A blank value means "no profile", like an empty `AWS_PROFILE`. `RequestOverride` keeps the
/// ambient chain, since the client applies that override to the final request itself.
pub(super) fn profile_from_auth(auth: &AuthData) -> Result<ProfileKey> {
	let profile = match auth {
		AuthData::None | AuthData::RequestOverride { .. } => None,
		AuthData::MultiKeys(keys) => Some(
			keys.get(PROFILE_AUTH_KEY)
				.ok_or_else(|| aws_err(format!("AuthData::MultiKeys requires a '{PROFILE_AUTH_KEY}' entry")))?
				.to_owned(),
		),
		// The single-valued forms (`Key`, `FromEnv`).
		single => Some(
			single
				.single_key_value()
				.map_err(|err| aws_err(format!("AWS profile lookup failed: {err}")))?,
		),
	};

	Ok(normalize_profile(profile.as_deref()))
}

/// Loads `aws-config`'s chain for `profile`, returning `(provider, region)`.
async fn load_aws_config(profile: Option<&str>) -> Result<(SharedCredentialsProvider, String)> {
	use aws_config::BehaviorVersion;

	let mut loader = aws_config::defaults(BehaviorVersion::latest());
	if let Some(profile) = profile {
		loader = loader.profile_name(profile);
	}
	let config = loader.load().await;

	let region = config
		.region()
		.map(|r| r.as_ref().to_string())
		.or_else(region_from_env)
		.unwrap_or_else(|| DEFAULT_REGION.to_string());

	let provider = config
		.credentials_provider()
		.ok_or_else(|| aws_err("AWS credentials (no provider found in default chain)"))?;

	Ok((provider, region))
}

/// Credential-resolution failure, naming the profile when the caller selected one.
fn creds_err(profile: Option<&str>, err: impl std::fmt::Display) -> Error {
	let for_profile = profile.map(|profile| format!(" for profile '{profile}'")).unwrap_or_default();
	aws_err(format!("AWS credential resolution failed{for_profile}: {err}"))
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
	let end = without_scheme.find(['/', ':', '?']).unwrap_or(without_scheme.len());
	let host = &without_scheme[..end];
	if host.is_empty() { None } else { Some(host) }
}

#[cfg(test)]
mod tests {
	use super::*;
	use aws_credential_types::provider;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};
	use tokio::sync::Notify;

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

	#[test]
	fn blank_profile_selects_the_ambient_chain() {
		assert_eq!(normalize_profile(None), None);
		// An empty `AWS_PROFILE` is ignored by the AWS CLI and aws-config, so a blank value here
		// must not become a profile named "".
		assert_eq!(normalize_profile(Some("")), None);
		assert_eq!(normalize_profile(Some("   ")), None);
		assert_eq!(normalize_profile(Some(" dev ")), Some("dev".to_string()));
	}

	#[test]
	fn profile_comes_from_the_auth_data() {
		assert_eq!(profile_from_auth(&AuthData::None).unwrap(), None);
		assert_eq!(
			profile_from_auth(&AuthData::Key("dev".to_string())).unwrap(),
			Some("dev".to_string())
		);
		assert_eq!(profile_from_auth(&AuthData::Key(" ".to_string())).unwrap(), None);
		assert_eq!(
			profile_from_auth(&AuthData::from_multi(HashMap::from([(
				PROFILE_AUTH_KEY.to_string(),
				"prod".to_string()
			)])))
			.unwrap(),
			Some("prod".to_string())
		);
	}

	#[test]
	fn multi_keys_without_a_profile_is_an_error() {
		let auth = AuthData::from_multi(HashMap::new());
		assert!(profile_from_auth(&auth).is_err(), "a profile key is required");
	}

	#[test]
	fn request_override_keeps_the_ambient_chain() {
		// The client applies `RequestOverride` to the final request, so it selects no profile here.
		let auth = AuthData::RequestOverride {
			url: String::new(),
			headers: Headers::default(),
		};
		assert_eq!(profile_from_auth(&auth).unwrap(), None);
	}

	#[test]
	fn env_auth_reads_the_profile_from_the_named_variable() {
		// `PATH` is set in every environment, so this covers the env-name path without mutating the
		// process environment (which the crate's `unsafe_code = "forbid"` lint rules out).
		let value = std::env::var("PATH").expect("PATH must be set");

		let profile = profile_from_auth(&AuthData::from_env("PATH")).expect("PATH is set");

		let expected = value.trim();
		let expected = (!expected.is_empty()).then(|| expected.to_owned());
		assert_eq!(profile, expected);
	}

	#[test]
	fn env_auth_for_a_missing_variable_is_an_error() {
		let auth = AuthData::from_env("GENAI_TEST_NO_SUCH_PROFILE_ENV");
		assert!(profile_from_auth(&auth).is_err());
	}

	// region:    --- Errors

	#[test]
	fn credential_errors_name_the_profile() {
		let named = creds_err(Some("dev"), "boom");
		assert!(named.to_string().contains("profile 'dev'"), "{named}");
		assert!(named.to_string().contains("boom"), "{named}");

		// Nothing to name when the ambient chain was used.
		let ambient = creds_err(None, "boom");
		assert!(!ambient.to_string().contains("profile"), "{ambient}");
	}

	// endregion: --- Errors

	// region:    --- Cache behavior

	/// Hands out fixed credentials and counts how often it was asked, so a test can tell a cached
	/// credential from a freshly fetched one.
	#[derive(Debug)]
	struct CountingProvider {
		access_key: &'static str,
		calls: Arc<AtomicUsize>,
	}

	impl ProvideCredentials for CountingProvider {
		fn provide_credentials<'a>(&'a self) -> provider::future::ProvideCredentials<'a>
		where
			Self: 'a,
		{
			self.calls.fetch_add(1, Ordering::SeqCst);
			let creds = Credentials::new(self.access_key, "secret", None, None, "genai-test");
			provider::future::ProvideCredentials::ready(Ok(creds))
		}
	}

	/// Inserts a fresh cache entry, so cache behavior is testable without touching AWS.
	fn seed(
		profile: Option<&str>,
		access_key: &'static str,
		region: &'static str,
		calls: &Arc<AtomicUsize>,
	) -> ProfileKey {
		let provider = SharedCredentialsProvider::new(CountingProvider {
			access_key,
			calls: calls.clone(),
		});

		seed_entry(profile, access_key, region, provider)
	}

	/// Inserts a fresh cache entry backed by `provider`.
	fn seed_entry(
		profile: Option<&str>,
		access_key: &'static str,
		region: &'static str,
		provider: SharedCredentialsProvider,
	) -> ProfileKey {
		let key = normalize_profile(profile);
		store(
			key.clone(),
			CachedCreds {
				creds: Credentials::new(access_key, "secret", None, None, "genai-test"),
				region: region.to_string(),
				provider,
				deadline: deadline(None, SystemTime::now(), 0.0),
			},
		);

		key
	}

	/// Expires an entry, instead of waiting out its TTL.
	fn expire(key: &ProfileKey) {
		let mut cached = cached_for(key).expect("the entry was just seeded");
		cached.deadline = SystemTime::UNIX_EPOCH;
		store(key.clone(), cached);
	}

	#[tokio::test]
	async fn fresh_credentials_are_served_from_the_cache() {
		let calls = Arc::new(AtomicUsize::new(0));
		seed(Some("test-fresh"), "AKIACACHED", "eu-west-1", &calls);

		let cached = get_credentials(Some("test-fresh")).await.expect("cached credentials");

		assert_eq!(cached.creds.access_key_id(), "AKIACACHED");
		assert_eq!(cached.region, "eu-west-1");
		assert_eq!(calls.load(Ordering::SeqCst), 0, "a fresh entry must not re-fetch");
	}

	#[tokio::test]
	async fn expired_credentials_are_refreshed_in_place() {
		let calls = Arc::new(AtomicUsize::new(0));
		let key = seed(Some("test-refresh"), "AKIAOLD", "eu-west-1", &calls);
		expire(&key);

		let refreshed = get_credentials(Some("test-refresh")).await.expect("refreshed credentials");

		assert_eq!(calls.load(Ordering::SeqCst), 1, "an expired entry must re-fetch");
		assert_eq!(
			refreshed.creds.access_key_id(),
			"AKIAOLD",
			"the cached provider is reused"
		);
		assert_eq!(refreshed.region, "eu-west-1", "the profile's region survives a refresh");
	}

	#[tokio::test]
	async fn each_profile_keeps_its_own_credentials() {
		let calls = Arc::new(AtomicUsize::new(0));
		seed(Some("test-dev"), "AKIADEV", "eu-west-1", &calls);
		seed(Some("test-prod"), "AKIAPROD", "us-east-1", &calls);

		let dev = get_credentials(Some("test-dev")).await.expect("dev credentials");
		let prod = get_credentials(Some("test-prod")).await.expect("prod credentials");

		assert_eq!(dev.creds.access_key_id(), "AKIADEV");
		assert_eq!(prod.creds.access_key_id(), "AKIAPROD");
		assert_ne!(dev.region, prod.region, "profiles must not share a region");
		assert_eq!(calls.load(Ordering::SeqCst), 0, "seeded entries need no fetch");
	}

	/// Parks inside `provide_credentials` until released, to hold one fetch in flight.
	#[derive(Debug)]
	struct BlockingProvider {
		entered: Arc<Notify>,
		release: Arc<Notify>,
	}

	impl ProvideCredentials for BlockingProvider {
		fn provide_credentials<'a>(&'a self) -> provider::future::ProvideCredentials<'a>
		where
			Self: 'a,
		{
			let entered = self.entered.clone();
			let release = self.release.clone();

			provider::future::ProvideCredentials::new(async move {
				entered.notify_one();
				release.notified().await;
				Ok(Credentials::new("AKIASLOW", "secret", None, None, "genai-test"))
			})
		}
	}

	/// The lock is never held across a fetch, so a fetch in flight must not stall another profile.
	#[tokio::test]
	async fn a_slow_fetch_does_not_block_another_profile() {
		let entered = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());

		let slow_key = seed_entry(
			Some("test-slow"),
			"AKIASLOW",
			"eu-west-1",
			SharedCredentialsProvider::new(BlockingProvider {
				entered: entered.clone(),
				release: release.clone(),
			}),
		);
		expire(&slow_key);

		let calls = Arc::new(AtomicUsize::new(0));
		seed(Some("test-other"), "AKIAFAST", "us-east-1", &calls);

		let slow = tokio::spawn(async move { get_credentials(Some("test-slow")).await });
		entered.notified().await;

		let other = tokio::time::timeout(Duration::from_secs(5), get_credentials(Some("test-other")))
			.await
			.expect("another profile must resolve while a fetch is in flight")
			.expect("the cached credentials should be usable");
		assert_eq!(other.creds.access_key_id(), "AKIAFAST");

		release.notify_one();
		slow.await.expect("task should not panic").expect("slow profile should resolve");
	}

	// endregion: --- Cache behavior
}
