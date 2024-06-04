use crate::webc::WebClient;
use crate::{EnvName, LegacyClientConfig};
use crate::{Error, Result};

pub(in crate::providers) trait Provider {
	/// Required - Return the provider inner
	fn provider_inner(&self) -> &ProviderInner;

	/// To be implemented by the Provider type,
	/// By default, return None
	fn default_api_key_env_name() -> Option<&'static str> {
		None
	}

	fn web_client(&self) -> &WebClient {
		&self.provider_inner().web_client
	}

	fn config(&self) -> Option<&LegacyClientConfig> {
		self.provider_inner().config.as_ref()
	}
}

// region:    --- ProviderInner

/// The common provider inner for all native providers.
pub struct ProviderInner {
	pub(in crate::providers) web_client: WebClient,
	#[allow(unused)] // for now, we do not use it
	pub(in crate::providers) config: Option<LegacyClientConfig>,
}

// endregion: --- ProviderInner

pub(crate) fn get_api_key_from_config(
	config: Option<&LegacyClientConfig>,
	default_env_name: Option<&'static str>,
) -> Result<String> {
	// -- First we try to ket it from the `key` property
	if let Some(key) = config.and_then(|c| c.key.as_ref()) {
		return Ok(key.clone());
	}

	// -- If not found, we look on the environment
	let env_name = match config.and_then(|c| c.key_from_env.as_ref()) {
		// if there is a named env name, then, return it
		Some(EnvName::Named(name)) => name.as_ref(),

		// otherwise, try to get the default name
		// for now, if key_from_env is None or ProviderDefault, same logic
		Some(EnvName::ProviderDefault) | None => {
			if let Some(name) = default_env_name {
				name
			} else {
				return Err(Error::ProviderHasNoDefaultApiKeyEnvName);
			}
		}
	};

	let key = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
		env_name: env_name.to_string(),
	})?;

	Ok(key)
}
