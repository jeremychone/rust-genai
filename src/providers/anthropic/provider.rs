use crate::providers::support::{Provider, ProviderInner};
use crate::webc::WebClient;
use crate::{LegacyClient, LegacyClientConfig, Result};
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1/";

#[derive(Clone)]
pub struct AnthropicProvider {
	inner: Arc<ProviderInner>,
}

impl AnthropicProvider {
	pub const DEFAULT_API_KEY_ENV_NAME: &'static str = "ANTHROPIC_API_KEY";
}

impl Provider for AnthropicProvider {
	fn provider_inner(&self) -> &ProviderInner {
		&self.inner
	}

	fn default_api_key_env_name() -> Option<&'static str> {
		Some(AnthropicProvider::DEFAULT_API_KEY_ENV_NAME)
	}
}

impl AnthropicProvider {
	pub fn default_client() -> impl LegacyClient {
		let inner = ProviderInner {
			web_client: WebClient::new().base_url(DEFAULT_BASE_URL),
			config: None,
		};
		Self { inner: inner.into() }
	}

	pub fn new_provider(_config: LegacyClientConfig) -> Result<Self> {
		todo!()
	}
}
