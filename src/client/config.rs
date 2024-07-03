use crate::chat::ChatRequestOptions;
use crate::resolver::AdapterKindResolver;

#[derive(Debug, Default)]
pub struct ClientConfig {
	adapter_kind_resolver: Option<AdapterKindResolver>,
	default_chat_request_options: Option<ChatRequestOptions>,
}

/// Adapter Related Setters (builder style)
impl ClientConfig {
	/// Set the built auth resolver
	pub fn with_auth_resolver(mut self, auth_resolver: AdapterKindResolver) -> Self {
		self.adapter_kind_resolver = Some(auth_resolver);
		self
	}

	/// Default chat request options
	pub fn with_default_chat_request_options(mut self, default_chat_request_options: ChatRequestOptions) -> Self {
		self.default_chat_request_options = Some(default_chat_request_options);
		self
	}
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn adapter_kind_resolver(&self) -> Option<&AdapterKindResolver> {
		self.adapter_kind_resolver.as_ref()
	}

	pub fn default_chat_request_options(&self) -> Option<&ChatRequestOptions> {
		self.default_chat_request_options.as_ref()
	}
}
