use crate::resolver::AuthResolver;

#[derive(Debug, Default)]
pub struct AdapterConfig {
	auth_resolver: Option<AuthResolver>,
}

/// Auth Related Chainable Setters
impl AdapterConfig {
	/// Set the built auth resolver
	pub fn with_auth_resolver(mut self, auth_resolver: AuthResolver) -> Self {
		self.auth_resolver = Some(auth_resolver);
		self
	}

	/// Convenient setter to set a AuthResolver from_env_name
	pub fn with_auth_env_name(mut self, auth_env_name: impl Into<String>) -> Self {
		let auth_env_name = auth_env_name.into();
		self.auth_resolver = Some(AuthResolver::from_env_name(auth_env_name));
		self
	}
}

/// Getters (as ref/deref)
impl AdapterConfig {
	pub fn auth_resolver(&self) -> Option<&AuthResolver> {
		self.auth_resolver.as_ref()
	}
}
