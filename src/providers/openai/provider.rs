use crate::{Client, Result};
use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::Client as AsyncOpenAIClient;
use std::sync::Arc;
use tokio::sync::Mutex;

// region:    --- Provider

/// async-openai provider
/// Note: for now, only support single chat completion mode (which is recommended for cost anyway)
#[derive(Clone)]
pub struct OpenAIProvider {
	pub(in crate::providers::openai) conn: Arc<Mutex<AsyncOaClient>>,
}

pub(in crate::providers::openai) type AsyncOaClient = AsyncOpenAIClient<AsyncOpenAIConfig>;

// Constructors
impl OpenAIProvider {
	/// Returns the client trait implementation.
	pub fn new_client(config: OpenAIProviderConfig) -> Result<impl Client> {
		OpenAIProvider::new(config)
	}

	/// Returns the raw Provider
	pub fn new(config: OpenAIProviderConfig) -> Result<Self> {
		let config = AsyncOpenAIConfig::new().with_api_key(config.api_key);
		let conn = Arc::new(Mutex::new(AsyncOpenAIClient::with_config(config)));
		Ok(Self { conn })
	}

	/// Returns the client trait implementation.
	pub fn client_from_api_key(api_key: String) -> Result<impl Client> {
		OpenAIProvider::from_api_key(api_key)
	}

	/// Returns the raw Provider
	pub fn from_api_key(api_key: String) -> Result<Self> {
		let config = OpenAIProviderConfig { api_key };
		Self::new(config)
	}

	/// Returns the client trait implementation.
	pub fn client_from_async_openai_client(async_client: AsyncOaClient) -> Self {
		OpenAIProvider::from_async_openai_client(async_client)
	}

	pub fn from_async_openai_client(async_client: AsyncOaClient) -> Self {
		let conn = Arc::new(Mutex::new(async_client));
		Self { conn }
	}

	pub fn client_from_async_openai_config(config: AsyncOpenAIConfig) -> Self {
		OpenAIProvider::from_async_openai_config(config)
	}

	// From async-openai config config
	pub fn from_async_openai_config(config: AsyncOpenAIConfig) -> Self {
		let conn = Arc::new(Mutex::new(AsyncOpenAIClient::with_config(config)));
		Self { conn }
	}
}

// endregion: --- Provider

// region:    --- OpenAIConfig

pub struct OpenAIProviderConfig {
	pub(super) api_key: String,
}

impl std::fmt::Debug for OpenAIProviderConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OpenaiClientConfig").field("api_key", &"REDACTED").finish()
	}
}

// endregion: --- OpenAIConfig
