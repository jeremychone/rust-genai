use crate::resolver::AuthResolver;

#[derive(Debug, Default)]
pub struct AdapterConfig {
	auth_resolver: Option<AuthResolver>,
}

/// Setter (builder style)
impl AdapterConfig {
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
