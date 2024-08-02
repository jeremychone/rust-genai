//! An AuthResolver is responsible for returning the AuthData (typically containing the `api_key`).
//! It can take the following forms:
//! - Configured with a custom environment name,
//! - Contain a fixed auth value,
//! - Contain an `AuthResolverFnSync` trait object or closure that will be called to return the AuthData.
//!
//! Note: AuthData is typically a single value but can be Multi for future adapters (e.g., AWS Bedrock).

use crate::adapter::AdapterKind;
use crate::resolver::{Error, Result};
use crate::ConfigSet;
use std::collections::HashMap;
use std::sync::Arc;

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

	pub fn from_sync_resolver(resolver_fn: impl IntoSyncAuthResolverFn) -> Self {
		AuthResolver {
			inner: AuthResolverInner::SyncResolverFn(resolver_fn.into_sync_resolver_fn()),
		}
	}
}

// region:    --- AuthDataProvider & IntoAuthDataProvider

pub trait SyncAuthResolverFn: Send + Sync {
	fn exec_sync_resolver_fn(&self, adapter_kind: AdapterKind, config_set: &ConfigSet) -> Result<Option<AuthData>>;
}

// Define a trait for types that can be converted into Arc<dyn AuthDataProviderSync>
pub trait IntoSyncAuthResolverFn {
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAuthResolverFn>;
}

// Implement IntoProvider for Arc<dyn AuthDataProviderSync>
impl IntoSyncAuthResolverFn for Arc<dyn SyncAuthResolverFn> {
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAuthResolverFn> {
		self
	}
}

// Implement IntoProvider for closures
impl<F> IntoSyncAuthResolverFn for F
where
	F: Fn(AdapterKind, &ConfigSet) -> Result<Option<AuthData>> + Send + Sync + 'static,
{
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAuthResolverFn> {
		Arc::new(self)
	}
}

// Implement AuthDataProviderSync for closures
impl<F> SyncAuthResolverFn for F
where
	F: Fn(AdapterKind, &ConfigSet) -> Result<Option<AuthData>> + Send + Sync,
{
	fn exec_sync_resolver_fn(&self, adapter_kind: AdapterKind, config_set: &ConfigSet) -> Result<Option<AuthData>> {
		self(adapter_kind, config_set)
	}
}

// endregion: --- AuthDataProvider & IntoAuthDataProvider

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
			AuthResolverInner::SyncResolverFn(sync_provider) => {
				sync_provider.exec_sync_resolver_fn(adapter_kind, config_set)
			}
		}
	}
}

enum AuthResolverInner {
	EnvName(String),
	Fixed(AuthData),
	SyncResolverFn(Arc<dyn SyncAuthResolverFn>),
}

// impl debug for AuthResolverInner
impl std::fmt::Debug for AuthResolverInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			AuthResolverInner::EnvName(env_name) => write!(f, "AuthResolverInner::EnvName({})", env_name),
			AuthResolverInner::Fixed(auth_data) => write!(f, "AuthResolverInner::Fixed({:?})", auth_data),
			AuthResolverInner::SyncResolverFn(_) => write!(f, "AuthResolverInner::FnSync(...)"),
		}
	}
}

// region:    --- AuthData

#[derive(Clone)]
pub enum AuthData {
	Single(String),
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
