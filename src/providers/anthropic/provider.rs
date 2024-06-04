use crate::webc::WebClient;
use crate::{Client, ClientConfig, Result};
use std::sync::Arc;

const DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1/";

#[derive(Clone)]
pub struct AnthropicProvider {
	inner: Arc<Inner>,
}

impl AnthropicProvider {
	pub const DEFAULT_API_KEY_ENV_NAME: &'static str = "ANTHROPIC_API_KEY";
}

struct Inner {
	web_client: WebClient,
	#[allow(unused)] // for now, we do not use it
	config: Option<ClientConfig>,
}

impl AnthropicProvider {
	pub(in crate::providers::anthropic) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}

	pub(in crate::providers::anthropic) fn config(&self) -> Option<&ClientConfig> {
		self.inner.config.as_ref()
	}
}

impl AnthropicProvider {
	pub fn default_client() -> impl Client {
		let inner = Inner {
			web_client: WebClient::new().base_url(DEFAULT_BASE_URL),
			config: None,
		};
		Self { inner: inner.into() }
	}

	pub fn new_provider(config: ClientConfig) -> Result<Self> {
		todo!()
	}
}
