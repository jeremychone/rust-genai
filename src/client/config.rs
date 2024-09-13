use crate::chat::ChatOptions;
use crate::resolver::{AuthResolver, ModelMapper};

/// The Client configuration used in the configuration builder stage.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(in crate::client) auth_resolver: Option<AuthResolver>,
	pub(in crate::client) model_mapper: Option<ModelMapper>,
	pub(in crate::client) chat_options: Option<ChatOptions>,
}

/// Adapter-related chainable setters for ClientConfig.
impl ClientConfig {
	/// Set the AuthResolver for the ClientConfig.
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		self.auth_resolver = Some(auth_resolver);
		self
	}

	/// Set the ModelMapper for the ClientConfig.
	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		self.model_mapper = Some(model_mapper);
		self
	}

	/// Set the default chat request options for the ClientConfig.
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}
}

/// Getters for ClientConfig fields (as references).
impl ClientConfig {
	/// Get a reference to the AuthResolver, if it exists.
	pub fn auth_resolver(&self) -> Option<&AuthResolver> {
		self.auth_resolver.as_ref()
	}

	/// Get a reference to the ModelMapper, if it exists.
	pub fn model_mapper(&self) -> Option<&ModelMapper> {
		self.model_mapper.as_ref()
	}

	/// Get a reference to the ChatOptions, if they exist.
	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}
}
