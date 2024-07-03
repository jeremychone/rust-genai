use crate::adapter::{AdapterConfig, AdapterKind};
use crate::chat::ChatRequestOptions;
use crate::client::ClientConfig;
use crate::webc::WebClient;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Client {
	inner: Arc<ClientInner>,
}

// region:    --- Client Construstors

impl Client {
	pub fn builder() -> ClientBuilder {
		ClientBuilder::default()
	}
}

impl Default for Client {
	fn default() -> Self {
		Client::builder().build()
	}
}

// endregion: --- Client Construstors

// region:    --- Client Getters

impl Client {
	pub(crate) fn web_client(&self) -> &WebClient {
		&self.inner.web_client
	}

	#[allow(unused)]
	pub(crate) fn config(&self) -> &ClientConfig {
		&self.inner.config
	}

	/// Returns the eventual custom AdapterConfig that has been set for this client (in the builder phase)
	pub(crate) fn custom_adapter_config(&self, adapter_kind: AdapterKind) -> Option<&AdapterConfig> {
		self.inner.apapter_config_by_kind.as_ref()?.get(&adapter_kind)
	}
}

// endregion: --- Client Getters

// region:    --- ClientBuilder

#[derive(Debug, Default)]
pub struct ClientBuilder {
	apapter_config_by_kind: Option<HashMap<AdapterKind, AdapterConfig>>,

	web_client: Option<WebClient>,

	#[allow(unused)] // for now, we do not use it
	config: Option<ClientConfig>,
}

/// Builder methods
impl ClientBuilder {
	pub fn with_reqwest(mut self, reqwest_client: reqwest::Client) -> Self {
		self.web_client = Some(WebClient::from_reqwest_client(reqwest_client));
		self
	}

	/// With a client config.
	pub fn with_config(mut self, config: ClientConfig) -> Self {
		self.config = Some(config);
		self
	}

	pub fn with_adapter_config(mut self, kind: AdapterKind, adapter_config: AdapterConfig) -> Self {
		self.apapter_config_by_kind
			.get_or_insert_with(HashMap::new)
			.insert(kind, adapter_config);
		self
	}

	/// Set ChatRequestOptions that will be used when executing each request.
	/// Each property (e.g., `temperature`, `max_tokens`) will be used if defined.
	/// The per-request ChatRequestOptions properties will take precedence if set.
	pub fn with_default_chat_request_options(mut self, default_chat_request_options: ChatRequestOptions) -> Self {
		self.config = Some(ClientConfig::default().with_default_chat_request_options(default_chat_request_options));
		self
	}

	pub fn build(self) -> Client {
		let inner = ClientInner {
			web_client: self.web_client.unwrap_or_default(),
			config: self.config.unwrap_or_default(),
			apapter_config_by_kind: self.apapter_config_by_kind,
		};
		Client { inner: Arc::new(inner) }
	}
}

// endregion: --- ClientBuilder

// region:    --- ClientInner

#[derive(Debug)]
struct ClientInner {
	#[allow(unused)]
	apapter_config_by_kind: Option<HashMap<AdapterKind, AdapterConfig>>,

	web_client: WebClient,

	config: ClientConfig,
}

// endregion: --- ClientInner
