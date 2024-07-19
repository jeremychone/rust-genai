use crate::adapter::{AdapterConfig, AdapterKind};
use crate::client::ClientConfig;
use crate::webc::WebClient;
use crate::ClientBuilder;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client {
	pub(super) inner: Arc<ClientInner>,
}

// region:    --- Client Construstors

impl Default for Client {
	fn default() -> Self {
		Client::builder().build()
	}
}

impl Client {
	pub fn builder() -> ClientBuilder {
		ClientBuilder::default()
	}
}

// endregion: --- Client Construstors

// region:    --- Client Getters

impl Client {
	pub(crate) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}

	pub(crate) fn config(&self) -> &ClientConfig {
		&self.inner.config
	}

	/// Returns the eventual custom AdapterConfig that has been set for this client (in the builder phase)
	pub(crate) fn custom_adapter_config(&self, adapter_kind: AdapterKind) -> Option<&AdapterConfig> {
		self.inner.apapter_config_by_kind.as_ref()?.get(&adapter_kind)
	}
}

// endregion: --- Client Getters

// region:    --- ClientInner

#[derive(Debug)]
pub(super) struct ClientInner {
	#[allow(unused)]
	pub(super) apapter_config_by_kind: Option<HashMap<AdapterKind, AdapterConfig>>,

	pub(super) web_client: WebClient,

	pub(super) config: ClientConfig,
}

// endregion: --- ClientInner
