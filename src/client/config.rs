use crate::resolver::AdapterKindResolver;

#[derive(Debug, Default)]
pub struct ClientConfig {
	adapter_kind_resolver: Option<AdapterKindResolver>,
}

/// Adapter Related Setters (builder style)
impl ClientConfig {
	/// Set the built auth resolver
	pub fn with_auth_resolver(mut self, auth_resolver: AdapterKindResolver) -> Self {
		self.adapter_kind_resolver = Some(auth_resolver);
		self
	}
}

/// Getters (as ref/deref)
impl ClientConfig {
	pub fn adapter_kind_resolver(&self) -> Option<&AdapterKindResolver> {
		self.adapter_kind_resolver.as_ref()
	}
}
