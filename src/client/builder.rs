use crate::adapter::{AdapterConfig, AdapterKind};
use crate::chat::ChatOptions;
use crate::resolver::AdapterKindResolver;
use crate::webc::WebClient;
use crate::{Client, ClientConfig};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct ClientBuilder {
	apapter_config_by_kind: Option<HashMap<AdapterKind, AdapterConfig>>,

	web_client: Option<WebClient>,

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

	pub fn insert_adapter_config(mut self, kind: AdapterKind, adapter_config: AdapterConfig) -> Self {
		self.apapter_config_by_kind
			.get_or_insert_with(HashMap::new)
			.insert(kind, adapter_config);
		self
	}
}

/// Builder ClientConfig passthrough convenient setters
/// The goal of those functions is to set nested value such as ClientConfig.
impl ClientBuilder {
	/// Set the ChatOptions for the ClientConfig of this ClientBuilder.
	/// Will create the ClientConfig if not present.
	/// Otherwise, will just set the `client_config.chat_options`
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.chat_options = Some(options);
		self
	}

	/// Set the AdapterKindResolver for the ClientConfig of this ClientBuilder.
	/// Will create the ClientConfig if not present.
	/// Otherwise, will just set the `client_config.adapter_kind_resolver`
	pub fn with_adapter_kind_resolver(mut self, resolver: AdapterKindResolver) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.adapter_kind_resolver = Some(resolver);
		self
	}
}

/// Build() method
impl ClientBuilder {
	pub fn build(self) -> Client {
		let inner = super::ClientInner {
			web_client: self.web_client.unwrap_or_default(),
			config: self.config.unwrap_or_default(),
			apapter_config_by_kind: self.apapter_config_by_kind,
		};
		Client { inner: Arc::new(inner) }
	}
}
