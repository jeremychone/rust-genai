use crate::client::{ClientConfig, EnvName};
use crate::{Error, Result};

/// NOTE: Later will have to get the client_config and the adapter_config
pub(crate) fn get_api_key_from_config(
	client_config: Option<&ClientConfig>,
	default_env_name: Option<&'static str>,
) -> Result<Option<String>> {
	// -- First we try to ket it from the `key` property
	if let Some(key) = client_config.and_then(|c| c.api_key.as_ref()) {
		return Ok(Some(key.clone()));
	}

	// -- If not found, we look on the environment
	let env_name = match client_config.and_then(|c| c.api_key_from_env.as_ref()) {
		// if there is a named env name, then, return it
		Some(EnvName::Named(name)) => name.as_ref(),

		// otherwise, try to get the default name
		// for now, if key_from_env is None or ProviderDefault, same logic
		Some(EnvName::AdapterDefault) => {
			if let Some(name) = default_env_name {
				name
			} else {
				return Err(Error::AdapterHasNoDefaultApiKeyEnvName);
			}
		}
		None => match default_env_name {
			Some(name) => name,
			None => return Ok(None),
		},
	};

	let key = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
		env_name: env_name.to_string(),
	})?;

	Ok(Some(key))
}
