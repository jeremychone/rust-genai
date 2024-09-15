use crate::adapter::{Adapter, AdapterDispatcher, AdapterKind};
use crate::resolver::AuthData;
use crate::{ClientConfig, ModelIden};
use crate::{Error, Result};

/// Returns the `api_key` value from the config_set auth_resolver
/// This function should be called if the adapter requires an api_key
/// Fails if there is no auth_resolver or no auth_data
pub fn get_api_key(model_iden: ModelIden, client_config: &ClientConfig) -> Result<String> {
	// -- Try to get it from the optional auth_resolver
	let auth_data = client_config
		.auth_resolver()
		.map(|auth_resolver| {
			auth_resolver
				.resolve(model_iden.clone())
				.map_err(|resolver_error| Error::Resolver {
					model_iden: model_iden.clone(),
					resolver_error,
				})
		})
		.transpose()? // return error if there is an error on auth resolver
		.flatten(); // flatten the two options

	// -- If there is no auth resolver, get it from the environment name (or 'ollama' for Ollama)
	let auth_data = auth_data.or_else(|| {
		if let AdapterKind::Ollama = model_iden.adapter_kind {
			Some(AuthData::from_single("ollama".to_string()))
		} else {
			AdapterDispatcher::default_key_env_name(model_iden.adapter_kind)
				.map(|env_name| AuthData::FromEnv(env_name.to_string()))
		}
	});

	let Some(auth_data) = auth_data else {
		return Err(Error::NoAuthData { model_iden });
	};

	// TODO: Needs to support multiple values
	let key = auth_data
		.single_value()
		.map_err(|resolver_error| Error::Resolver {
			model_iden,
			resolver_error,
		})?
		.to_string();

	Ok(key)
}