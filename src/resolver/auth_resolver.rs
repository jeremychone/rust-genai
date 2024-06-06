use crate::adapter::AdapterKind;
use crate::ConfigSet;
use crate::{Error, Result};
use std::collections::HashMap;

type SyncFnType = Box<dyn Fn(AdapterKind, &ConfigSet) -> Result<Option<AuthData>>>;

#[derive(Debug)]
pub struct AuthResolver {
	inner: AuthResolverInner,
}

impl AuthResolver {
	pub fn from_env_name(env_name: impl Into<String>) -> Self {
		AuthResolver {
			inner: AuthResolverInner::EnvName(env_name.into()),
		}
	}

	pub fn from_key_value(key: impl Into<String>) -> Self {
		AuthResolver {
			inner: AuthResolverInner::Fixed(AuthData::from_single(key)),
		}
	}

	// todo more
}

impl AuthResolver {
	pub(crate) fn resolve(&self, adapter_kind: AdapterKind, config_set: &ConfigSet) -> Result<Option<AuthData>> {
		match &self.inner {
			AuthResolverInner::EnvName(env_name) => {
				let key = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
					env_name: env_name.to_string(),
				})?;
				Ok(Some(AuthData::from_single(key)))
			}
			AuthResolverInner::Fixed(auth_data) => Ok(Some(auth_data.clone())),
			AuthResolverInner::SyncFn(sync_fn) => sync_fn(adapter_kind, config_set),
		}
	}
}

enum AuthResolverInner {
	EnvName(String),
	Fixed(AuthData),
	#[allow(unused)] // future
	SyncFn(SyncFnType),
}

// impl debug for AuthResolverInner
impl std::fmt::Debug for AuthResolverInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AuthResolverInner::EnvName(env_name) => write!(f, "AuthResolverInner::EnvName({})", env_name),
			AuthResolverInner::Fixed(auth_data) => write!(f, "AuthResolverInner::Fixed({:?})", auth_data),
			AuthResolverInner::SyncFn(_) => write!(f, "AuthResolverInner::SyncFn(...)"),
		}
	}
}

// region:    --- AuthData

#[derive(Clone)]
pub enum AuthData {
	Single(String),
	// TODO: Probable needs a HashMap
	Multi(HashMap<String, String>),
}

/// Constructors
impl AuthData {
	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Single(value.into())
	}

	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::Multi(data)
	}
}

/// Getters
impl AuthData {
	pub fn single_value(&self) -> Result<&str> {
		match self {
			AuthData::Single(value) => Ok(value.as_str()),
			AuthData::Multi(_) => Err(Error::ResolverAuthDataNotSingleValue),
		}
	}
}

// endregion: --- AuthData

// region:    --- AuthData Std Impls

// implement Debug to redact
impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AuthData::Single(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::Multi(_) => write!(f, "AuthData::Multi(REDACTED)"),
		}
	}
}

// endregion: --- AuthData Std Impls
