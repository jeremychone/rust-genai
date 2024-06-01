// region:    --- Modules

mod openai_impl;

use crate::Result;
use async_openai::config::OpenAIConfig as AsyncOpenAIConfig;
use async_openai::Client as AsyncOpenAIClient;
use std::sync::{Arc, Mutex};

// endregion: --- Modules

// region:    --- Client

pub type AsyncOaClient = AsyncOpenAIClient<AsyncOpenAIConfig>;

#[derive(Clone)]
pub struct OpenAIClient {
	conn: Arc<Mutex<AsyncOaClient>>,
}

// constructors
impl OpenAIClient {
	pub fn new(config: OpenAIConfig) -> Result<Self> {
		let config = AsyncOpenAIConfig::new().with_api_key(config.api_key);
		let conn = Arc::new(Mutex::new(AsyncOpenAIClient::with_config(config)));
		Ok(Self { conn })
	}

	pub fn from_api_key(api_key: String) -> Result<Self> {
		let config = OpenAIConfig { api_key };
		Self::new(config)
	}

	pub fn from_async_openai_client(async_client: AsyncOaClient) -> Self {
		let conn = Arc::new(Mutex::new(async_client));
		Self { conn }
	}

	// from config
	pub fn from_async_openai_config(config: AsyncOpenAIConfig) -> Self {
		let conn = Arc::new(Mutex::new(AsyncOpenAIClient::with_config(config)));
		Self { conn }
	}
}

// endregion: --- Client

// region:    --- OpenAIConfig

pub struct OpenAIConfig {
	api_key: String,
}
impl std::fmt::Debug for OpenAIConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OpenaiClientConfig").field("api_key", &"REDACTED").finish()
	}
}

// endregion: --- OpenAIConfig
