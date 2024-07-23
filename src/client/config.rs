use crate::chat::ChatOptions;
use crate::resolver::AdapterKindResolver;

// Note: Here the properties are `(in crate::client)` to allow the Client builder to set those values
//       with passthrough setters.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(in crate::client) adapter_kind_resolver: Option<AdapterKindResolver>,
	pub(in crate::client) chat_options: Option<ChatOptions>,
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
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn adapter_kind_resolver(&self) -> Option<&AdapterKindResolver> {
		self.adapter_kind_resolver.as_ref()
	}

	pub fn chat_options(&self) -> Option<&ChatOptions> {
		self.chat_options.as_ref()
	}
}
