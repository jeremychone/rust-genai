//! An AuthResolver is responsible for returning the AuthData (typically containing the `api_key`).
//! It can take the following forms:
//! - Configured with a custom environment name,
//! - Contain a fixed auth value,
//! - Contain an `AuthResolverFn` trait object or closure that will be called to return the AuthData.
//!
//! Note: AuthData is typically a single value but can be Multi for future adapters (e.g., AWS Bedrock).

use crate::resolver::{Error, Result};
use crate::{ClientConfig, ModelInfo};
use std::collections::HashMap;
use std::sync::Arc;

// region:    --- AuthResolver

/// Holder for the AuthResolver function.
/// NOTE: Eventually, we might also have a `ResolveAsyncFn`, hence the enum
#[derive(Debug, Clone)]
pub enum AuthResolver {
	ResolverFn(Arc<Box<dyn AuthResolverFn>>),
}

impl AuthResolver {
	pub fn from_resolver_fn(resolver_fn: impl IntoAuthResolverFn) -> Self {
		AuthResolver::ResolverFn(resolver_fn.into_resolver_fn())
	}
}

impl AuthResolver {
	pub(crate) fn resolve(&self, model_info: ModelInfo, client_config: &ClientConfig) -> Result<Option<AuthData>> {
		match self {
			AuthResolver::ResolverFn(resolver_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn
				resolver_fn.clone().exec_fn(model_info, client_config)
			}
		}
	}
}

// endregion: --- AuthResolver

// region:    --- AuthResolverFn

// Define the trait for an auth resolver function
pub trait AuthResolverFn: Send + Sync {
	fn exec_fn(&self, model_info: ModelInfo, client_config: &ClientConfig) -> Result<Option<AuthData>>;
	fn clone_box(&self) -> Box<dyn AuthResolverFn>;
}

// Implement AuthResolverFn for any `FnOnce`
impl<F> AuthResolverFn for F
where
	F: FnOnce(ModelInfo, &ClientConfig) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_info: ModelInfo, client_config: &ClientConfig) -> Result<Option<AuthData>> {
		(self.clone())(model_info, client_config)
	}

	fn clone_box(&self) -> Box<dyn AuthResolverFn> {
		Box::new(self.clone())
	}
}

// Implement Clone for Box<dyn AuthResolverFn>
impl Clone for Box<dyn AuthResolverFn> {
	fn clone(&self) -> Box<dyn AuthResolverFn> {
		self.clone_box()
	}
}

impl std::fmt::Debug for dyn AuthResolverFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AuthResolverFn")
	}
}

// endregion: --- AuthResolverFn

// region:    --- IntoAuthResolverFn

pub trait IntoAuthResolverFn {
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>>;
}

impl IntoAuthResolverFn for Arc<Box<dyn AuthResolverFn>> {
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		self
	}
}

// Implement IntoAuthResolverFn for closures
impl<F> IntoAuthResolverFn for F
where
	F: FnOnce(ModelInfo, &ClientConfig) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoAuthResolverFn

// region:    --- AuthData

#[derive(Clone)]
pub enum AuthData {
	FromEnv(String),
	Key(String),
	MultiKeys(HashMap<String, String>),
}

/// Constructors
impl AuthData {
	pub fn from_env(env_name: impl Into<String>) -> Self {
		AuthData::FromEnv(env_name.into())
	}
	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Key(value.into())
	}

	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::MultiKeys(data)
	}
}

/// Getters
impl AuthData {
	pub fn single_value(&self) -> Result<String> {
		match self {
			AuthData::FromEnv(env_name) => {
				// get value from env name
				let value = std::env::var(env_name).map_err(|_| Error::ApiKeyEnvNotFound {
					env_name: env_name.to_string(),
				})?;
				Ok(value)
			}
			AuthData::Key(value) => Ok(value.to_string()),
			AuthData::MultiKeys(_) => Err(Error::ResolverAuthDataNotSingleValue),
		}
	}
}

// endregion: --- AuthData

// region:    --- AuthData Std Impls

// implement Debug to redact
impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// NOTE: here we also redact for FromEnv in case dev confused this with key
			AuthData::FromEnv(_env_name) => write!(f, "AuthData::FromEnv(REDACTED)"),
			AuthData::Key(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::MultiKeys(_) => write!(f, "AuthData::Multi(REDACTED)"),
		}
	}
}

// endregion: --- AuthData Std Impls
