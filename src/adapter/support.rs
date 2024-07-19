use crate::adapter::AdapterKind;
use crate::ConfigSet;
use crate::{Error, Result};

/// Returns the `api_key` value from the config_set auth_resolver
/// This function should be called if the adapter must have a api_key
/// Fail if the no auth_resolver or no auth_data
pub(crate) fn get_api_key_resolver(adapter_kind: AdapterKind, config_set: &ConfigSet<'_>) -> Result<String> {
	let auth_resolver = config_set
		.adapter_config()
		.auth_resolver()
		.ok_or(Error::NoAuthResolver { adapter_kind })?;

	let auth_data = auth_resolver
		.resolve(adapter_kind, config_set)?
		.ok_or(Error::AuthResolverNoAuthData { adapter_kind })?;

	let key = auth_data.single_value()?.to_string();

	Ok(key)
}
