use crate::chat::ChatOptions;
use crate::resolver::AdapterKindResolver;

// Note: Here the properties are `(in crate::client)` to allow the Client builder to set those values
//       with passthrough setters.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(in crate::client) adapter_kind_resolver: Option<AdapterKindResolver>,
	pub(in crate::client) chat_options: Option<ChatOptions>,
	pub(in crate::client) default_model: Option<String>,
	pub(in crate::client) default_api_key: Option<String>,
}

/// Adapter Related Chainable Setters
impl ClientConfig {
	/// Set the built auth resolver
	pub fn with_adapter_kind_resolver(mut self, auth_resolver: AdapterKindResolver) -> Self {
		self.adapter_kind_resolver = Some(auth_resolver);
		self
	}

	/// Default chat request options
	pub fn with_chat_options(mut self, options: ChatOptions) -> Self {
		self.chat_options = Some(options);
		self
	}

	/// Default model to use for chat requests
	pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
		self.default_model = Some(model.into());
		self
	}

	/// Default API key to use for chat requests
	pub fn with_default_api_key(mut self, api_key: impl Into<String>) -> Self {
		self.default_api_key = Some(api_key.into());
		self
	}
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn adapter_kind_resolver(&self) -> Option<&AdapterKindResolver> {
		self.adapter_kind_resolver.as_ref()
	}

	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}

	pub fn default_model(&self) -> Option<&str> {
		self.default_model.as_deref()
	}

	pub fn default_api_key(&self) -> Option<&str> {
		self.default_api_key.as_deref()
	}
}
