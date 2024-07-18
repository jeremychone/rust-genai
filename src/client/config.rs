use crate::chat::ChatRequestOptions;
use crate::resolver::AdapterKindResolver;

// Note: Here the properties are `(in crate::client)` to allow the Client builder to set those values
//       with passthrough setters.
#[derive(Debug, Default, Clone)]
pub struct ClientConfig {
	pub(in crate::client) adapter_kind_resolver: Option<AdapterKindResolver>,
	pub(in crate::client) chat_request_options: Option<ChatRequestOptions>,
}

/// Adapter Related Chainable Setters
impl ClientConfig {
	/// Set the built auth resolver
	pub fn with_auth_resolver(mut self, auth_resolver: AdapterKindResolver) -> Self {
		self.adapter_kind_resolver = Some(auth_resolver);
		self
	}

	/// Default chat request options
	pub fn with_chat_request_options(mut self, default_chat_request_options: ChatRequestOptions) -> Self {
		self.chat_request_options = Some(default_chat_request_options);
		self
	}
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn adapter_kind_resolver(&self) -> Option<&AdapterKindResolver> {
		self.adapter_kind_resolver.as_ref()
	}

	pub fn chat_request_options(&self) -> Option<&ChatRequestOptions> {
		self.chat_request_options.as_ref()
	}
}
