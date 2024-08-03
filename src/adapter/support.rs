use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind};
use crate::resolver::AuthData;
use crate::{ClientConfig, ModelInfo};
use crate::{Error, Result};

/// Returns the `api_key` value from the config_set auth_resolver
/// This function should be called if the adapter must have a api_key
/// Fail if the no auth_resolver or no auth_data
pub fn get_api_key(model_info: ModelInfo, client_config: &ClientConfig) -> Result<String> {
	// -- Try to get it from the eventual auth_resolver
	let auth_data = client_config
		.auth_resolver()
		.map(|auth_resolver| {
			auth_resolver
				.resolve(model_info.clone(), client_config)
				.map_err(|resolver_error| Error::Resolver {
					model_info: model_info.clone(),
					resolver_error,
				})
		})
		.transpose()? // return error if error on auth resolver
		.flatten(); // flatten the two options

	// -- If no auth resolver, get it from env name (or 'ollama' for ollama)
	let auth_data = auth_data.or_else(|| {
		if let AdapterKind::Ollama = model_info.adapter_kind {
			Some(AuthData::from_single("ollama".to_string()))
		} else {
			AdapterDispatcher::default_key_env_name(model_info.adapter_kind)
				.map(|env_name| AuthData::FromEnv(env_name.to_string()))
		}
	});

	let Some(auth_data) = auth_data else {
		return Err(Error::NoAuthData { model_info });
	};

	// TODO: Needs to support multi value
	let key = auth_data
		.single_value()
		.map_err(|resolver_error| Error::Resolver {
			model_info,
			resolver_error,
		})?
		.to_string();

	Ok(key)
}
