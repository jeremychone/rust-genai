use crate::{LegacyClient, LegacyClientConfig, Result};
use async_openai as oa;
use async_openai::config as oac;
use std::sync::Arc;

type OaClient = oa::Client<oac::OpenAIConfig>;

/// async-openai provider
/// Note: for now, only support single chat completion mode (which is recommended for cost anyway)
#[derive(Debug, Default, Clone)]
pub struct OpenAIProvider {
	inner: Arc<Inner>,
}

impl OpenAIProvider {
	pub const DEFAULT_API_KEY_ENV_NAME: &'static str = "OPENAI_API_KEY";
}

#[derive(Debug)]
struct Inner {
	conn: OaClient,
	#[allow(unused)] // for now, we do not use it
	config: Option<LegacyClientConfig>,
}

// implement default
impl Default for Inner {
	fn default() -> Self {
		let conn = OaClient::new();
		Self { conn, config: None }
	}
}

impl OpenAIProvider {
	pub(in crate::providers::ext::ext_async_openai) fn conn(&self) -> &OaClient {
		&self.inner.conn
	}
	#[allow(unused)]
	pub(in crate::providers::ext::ext_async_openai) fn config(&self) -> Option<&LegacyClientConfig> {
		self.inner.config.as_ref()
	}
}

// Constructors
impl OpenAIProvider {
	pub fn default_client() -> impl LegacyClient {
		Self::default()
	}

	/// Returns the client trait implementation.
	pub fn new_client(config: LegacyClientConfig) -> Result<impl LegacyClient> {
		OpenAIProvider::new_provider(config)
	}

	/// Returns the raw Provider
	pub fn new_provider(config: LegacyClientConfig) -> Result<Self> {
		let oa_config: oac::OpenAIConfig = (&config).into();
		let conn = oa::Client::with_config(oa_config);
		let inner = Inner {
			config: Some(config),
			conn,
		};
		Ok(Self { inner: inner.into() })
	}

	/// Returns the client trait implementation.
	pub fn client_from_api_key(api_key: String) -> Result<impl LegacyClient> {
		OpenAIProvider::from_api_key(api_key)
	}

	/// Returns the raw Provider
	pub fn from_api_key(api_key: String) -> Result<Self> {
		let config = LegacyClientConfig::default().key(api_key);
		Self::new_provider(config)
	}

	// region:    --- Lower Level Constructors

	/// Returns the client trait implementation.
	pub fn client_from_async_openai_client(async_client: OaClient) -> Result<impl LegacyClient> {
		OpenAIProvider::from_async_openai_client(async_client)
	}

	pub fn from_async_openai_client(async_client: OaClient) -> Result<Self> {
		let conn = async_client;
		let inner = Inner { config: None, conn };
		Ok(Self { inner: inner.into() })
	}

	pub fn client_from_async_openai_config(config: oac::OpenAIConfig) -> Result<impl LegacyClient> {
		OpenAIProvider::from_async_openai_config(config)
	}

	// From async-openai config config
	pub fn from_async_openai_config(config: oac::OpenAIConfig) -> Result<Self> {
		let conn = oa::Client::with_config(config);
		let inner = Inner { config: None, conn };
		Ok(Self { inner: inner.into() })
	}

	// endregion: --- Lower Level Constructors
}
