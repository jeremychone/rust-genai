use crate::chat::ChatOptions;
use crate::resolver::{AuthResolver, IntoAuthResolverFn, IntoModelMapperFn, ModelMapper};
use crate::webc::WebClient;
use crate::{Client, ClientConfig};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct ClientBuilder {
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

	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.auth_resolver = Some(auth_resolver);
		self
	}

	pub fn with_auth_resolver_fn(mut self, auth_resolver_fn: impl IntoAuthResolverFn) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		let auth_resolver = AuthResolver::from_resolver_fn(auth_resolver_fn);
		client_config.auth_resolver = Some(auth_resolver);
		self
	}

	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.model_mapper = Some(model_mapper);
		self
	}

	pub fn with_model_mapper_fn(mut self, model_mapper_fn: impl IntoModelMapperFn) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		let model_mapper = ModelMapper::from_mapper_fn(model_mapper_fn);
		client_config.model_mapper = Some(model_mapper);
		self
	}
}

/// Build() method
impl ClientBuilder {
	pub fn build(self) -> Client {
		let inner = super::ClientInner {
			web_client: self.web_client.unwrap_or_default(),
			config: self.config.unwrap_or_default(),
		};
		Client { inner: Arc::new(inner) }
	}
}
