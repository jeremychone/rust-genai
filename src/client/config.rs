use crate::chat::ChatOptions;
use crate::resolver::{AuthResolver, ModelMapper};

// Note: Here the properties are `(in crate::client)` to allow the Client builder to set those values
//       with passthrough setters.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(in crate::client) auth_resolver: Option<AuthResolver>,
	pub(in crate::client) model_mapper: Option<ModelMapper>,
	pub(in crate::client) chat_options: Option<ChatOptions>,
}

/// Adapter Related Chainable Setters
impl ClientConfig {
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		self.auth_resolver = Some(auth_resolver);
		self
	}

	pub fn with_model_mapper(mut self, model_mapper: ModelMapper) -> Self {
		self.model_mapper = Some(model_mapper);
		self
	}

	/// Default chat request options
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn auth_resolver(&self) -> Option<&AuthResolver> {
		self.auth_resolver.as_ref()
	}

	pub fn model_mapper(&self) -> Option<&ModelMapper> {
		self.model_mapper.as_ref()
	}

	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}
}
