use crate::chat::ChatOptions;
use crate::resolver::{
	AuthResolver, IntoAuthResolverFn, IntoModelMapperFn, IntoServiceTargetResolverFn, ModelMapper,
	ServiceTargetResolver,
};
use crate::webc::WebClient;
use crate::{Client, ClientConfig, WebConfig};
use std::sync::Arc;

/// The builder for the `Client` structure.
///
/// - `ClientBuilder::default()`
/// - `Client::builder()`
#[derive(Debug, Default)]
pub struct ClientBuilder {
	web_client: Option<WebClient>,
	config: Option<ClientConfig>,
}

/// Builder methods
impl ClientBuilder {
	/// Create a new ClientBuilder with a custom `reqwest::Client`.
	pub fn with_reqwest(mut self, reqwest_client: reqwest::Client) -> Self {
		self.web_client = Some(WebClient::from_reqwest_client(reqwest_client));
		self
	}

	/// With a client configuration.
	pub fn with_config(mut self, config: ClientConfig) -> Self {
		self.config = Some(config);
		self
	}

	/// Set the reqwest configuration for the ClientConfig of this ClientBuilder.
	pub fn with_web_config(mut self, req_options: WebConfig) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.web_config = Some(req_options);
		self
	}
}

/// Builder ClientConfig passthrough convenient setters.
/// The goal of these functions is to set nested values such as ClientConfig.
impl ClientBuilder {
	/// Set the ChatOptions for the ClientConfig of this ClientBuilder.
	/// This will create the ClientConfig if it is not present.
	/// Otherwise, it will just set the `client_config.chat_options`.
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.chat_options = Some(options);
		self
	}

	/// Set the authentication resolver for the ClientConfig of this ClientBuilder.
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.auth_resolver = Some(auth_resolver);
		self
	}

	/// Set the authentication resolver function for the ClientConfig of this ClientBuilder.
	pub fn with_auth_resolver_fn(mut self, auth_resolver_fn: impl IntoAuthResolverFn) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		let auth_resolver = AuthResolver::from_resolver_fn(auth_resolver_fn);
		client_config.auth_resolver = Some(auth_resolver);
		self
	}

	pub fn with_service_target_resolver(mut self, target_resolver: ServiceTargetResolver) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.service_target_resolver = Some(target_resolver);
		self
	}

	pub fn with_service_target_resolver_fn(mut self, target_resolver_fn: impl IntoServiceTargetResolverFn) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		let target_resolver = ServiceTargetResolver::from_resolver_fn(target_resolver_fn);
		client_config.service_target_resolver = Some(target_resolver);
		self
	}

	/// Set the model mapper for the ClientConfig of this ClientBuilder.
	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		client_config.model_mapper = Some(model_mapper);
		self
	}

	/// Set the model mapper function for the ClientConfig of this ClientBuilder.
	pub fn with_model_mapper_fn(mut self, model_mapper_fn: impl IntoModelMapperFn) -> Self {
		let client_config = self.config.get_or_insert_with(ClientConfig::default);
		let model_mapper = ModelMapper::from_mapper_fn(model_mapper_fn);
		client_config.model_mapper = Some(model_mapper);
		self
	}
}

impl ClientBuilder {
	/// Build a new immutable GenAI client.
	pub fn build(self) -> Client {
		let config = self.config.unwrap_or_default();

		// Create WebClient based on configuration
		let web_client = if let Some(web_client) = self.web_client {
			// Use explicitly provided WebClient
			web_client
		} else if let Some(req_config) = config.web_config() {
			// Create WebClient with reqwest configuration
			let mut builder = reqwest::Client::builder();
			builder = req_config.apply_to_builder(builder);
			let reqwest_client = builder.build().expect("Failed to build reqwest client");
			WebClient::from_reqwest_client(reqwest_client)
		} else {
			// Use default WebClient
			WebClient::default()
		};

		let inner = super::ClientInner { web_client, config };
		Client { inner: Arc::new(inner) }
	}
}
