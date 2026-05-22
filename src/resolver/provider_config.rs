use crate::resolver::{AuthData, Endpoint};

/// Provider-level endpoint and auth overrides.
#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
	pub endpoint: Option<Endpoint>,
	pub auth: Option<AuthData>,
}

/// Constructors
impl ProviderConfig {
	pub fn from_endpoint(endpoint: impl Into<Endpoint>) -> Self {
		Self {
			endpoint: Some(endpoint.into()),
			auth: None,
		}
	}

	pub fn from_auth(auth: AuthData) -> Self {
		Self {
			endpoint: None,
			auth: Some(auth),
		}
	}
}

/// Chainable setters
impl ProviderConfig {
	pub fn with_endpoint(mut self, endpoint: impl Into<Endpoint>) -> Self {
		self.endpoint = Some(endpoint.into());
		self
	}

	pub fn with_auth(mut self, auth: AuthData) -> Self {
		self.auth = Some(auth);
		self
	}
}

// region:    --- ProviderConfig From Impls

impl From<()> for ProviderConfig {
	fn from(_: ()) -> Self {
		Self::default()
	}
}

impl From<Option<ProviderConfig>> for ProviderConfig {
	fn from(value: Option<ProviderConfig>) -> Self {
		value.unwrap_or_default()
	}
}

impl From<Endpoint> for ProviderConfig {
	fn from(endpoint: Endpoint) -> Self {
		Self {
			endpoint: Some(endpoint),
			auth: None,
		}
	}
}

impl From<AuthData> for ProviderConfig {
	fn from(auth: AuthData) -> Self {
		Self {
			endpoint: None,
			auth: Some(auth),
		}
	}
}

impl From<(Endpoint, AuthData)> for ProviderConfig {
	fn from((endpoint, auth): (Endpoint, AuthData)) -> Self {
		Self {
			endpoint: Some(endpoint),
			auth: Some(auth),
		}
	}
}

impl From<(Option<Endpoint>, Option<AuthData>)> for ProviderConfig {
	fn from((endpoint, auth): (Option<Endpoint>, Option<AuthData>)) -> Self {
		Self { endpoint, auth }
	}
}

// endregion: --- ProviderConfig From Impls

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;

	fn accepts_provider_config(_: impl Into<ProviderConfig>) {}

	#[test]
	fn none_infers_as_provider_config_default() {
		accepts_provider_config(None);

		let provider_config = ProviderConfig::from(None);
		assert!(provider_config.endpoint.is_none());
		assert!(provider_config.auth.is_none());
	}

	#[test]
	fn unit_maps_to_provider_config_default() {
		let provider_config = ProviderConfig::from(());
		assert!(provider_config.endpoint.is_none());
		assert!(provider_config.auth.is_none());
	}

	#[test]
	fn endpoint_maps_to_endpoint_only_provider_config() {
		let provider_config = ProviderConfig::from(Endpoint::from_static("http://example.com/"));
		assert_eq!(
			provider_config.endpoint.as_ref().map(Endpoint::base_url),
			Some("http://example.com/")
		);
		assert!(provider_config.auth.is_none());
	}

	#[test]
	fn auth_maps_to_auth_only_provider_config() {
		let provider_config = ProviderConfig::from(AuthData::None);
		assert!(provider_config.endpoint.is_none());
		assert!(matches!(provider_config.auth, Some(AuthData::None)));
	}

	#[test]
	fn tuple_maps_to_full_provider_config() {
		let provider_config = ProviderConfig::from((Endpoint::from_static("http://example.com/"), AuthData::None));
		assert_eq!(
			provider_config.endpoint.as_ref().map(Endpoint::base_url),
			Some("http://example.com/")
		);
		assert!(matches!(provider_config.auth, Some(AuthData::None)));
	}
}

// endregion: --- Tests
