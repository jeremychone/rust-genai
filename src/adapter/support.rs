use crate::{ConfigSet, ModelInfo};
use crate::{Error, Result};

/// Returns the `api_key` value from the config_set auth_resolver
/// This function should be called if the adapter must have a api_key
/// Fail if the no auth_resolver or no auth_data
pub fn get_api_key_resolver(model_info: ModelInfo, config_set: &ConfigSet<'_>) -> Result<String> {
	let auth_resolver = config_set
		.adapter_config()
		.auth_resolver()
		.ok_or_else(|| Error::NoAuthResolver {
			model_info: model_info.clone(),
		})?;

	let auth_data = auth_resolver
		.resolve(model_info.adapter_kind, config_set)
		.map_err(|resolver_error| Error::Resolver {
			model_info: model_info.clone(),
			resolver_error,
		})?
		.ok_or_else(|| Error::AuthResolverNoAuthData {
			model_info: model_info.clone(),
		})?;

	let key = auth_data
		.single_value()
		.map_err(|resolver_error| Error::Resolver {
			model_info,
			resolver_error,
		})?
		.to_string();

	Ok(key)
}
