//! An `AuthResolver` is responsible for returning the `AuthData` (typically containing the `api_key`).
//! It can take the following forms:
//! - Configured with a custom environment name,
//! - Contains a fixed auth value,
//! - Contains an `AuthResolverFn` trait object or closure that will be called to return the `AuthData`.
//!
//! Note: `AuthData` is typically a single value but can be multiple for future adapters (e.g., AWS Bedrock).

use crate::resolver::{Error, Result};
use crate::ModelIden;
use std::collections::HashMap;
use std::sync::Arc;

// region:    --- AuthResolver

/// Holder for the `AuthResolver` function.
/// NOTE: Eventually, we might also have a `ResolveAsyncFn`, hence the enum.
#[derive(Debug, Clone)]
pub enum AuthResolver {
	/// The `AuthResolverFn` trait object.
	ResolverFn(Arc<Box<dyn AuthResolverFn>>),
}

impl AuthResolver {
	/// Create a new `AuthResolver` from a resolver function.
	pub fn from_resolver_fn(resolver_fn: impl IntoAuthResolverFn) -> Self {
		AuthResolver::ResolverFn(resolver_fn.into_resolver_fn())
	}
}

impl AuthResolver {
	pub(crate) fn resolve(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		match self {
			AuthResolver::ResolverFn(resolver_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn.
				resolver_fn.clone().exec_fn(model_iden)
			}
		}
	}
}

// endregion: --- AuthResolver

// region:    --- AuthResolverFn

/// The `AuthResolverFn` trait object.
pub trait AuthResolverFn: Send + Sync {
	/// Execute the `AuthResolverFn` to get the `AuthData`.
	fn exec_fn(&self, model_iden: ModelIden) -> Result<Option<AuthData>>;

	/// Clone the trait object.
	fn clone_box(&self) -> Box<dyn AuthResolverFn>;
}

/// `AuthResolverFn` blanket implementation for any function that matches the `AuthResolver` function signature.
impl<F> AuthResolverFn for F
where
	F: FnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		(self.clone())(model_iden)
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

/// Custom and convenient trait used in the `AuthResolver::from_resolver_fn` argument.
pub trait IntoAuthResolverFn {
	/// Convert the argument into an `AuthResolverFn` trait object.
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>>;
}

impl IntoAuthResolverFn for Arc<Box<dyn AuthResolverFn>> {
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		self
	}
}

// Implement `IntoAuthResolverFn` for closures.
impl<F> IntoAuthResolverFn for F
where
	F: FnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoAuthResolverFn

// region:    --- AuthData

/// `AuthData` specifies either how or the key itself for an authentication resolver call.
#[derive(Clone)]
pub enum AuthData {
	/// Specify the environment name to get the key value from.
	FromEnv(String),

	/// The key value itself.
	Key(String),

	/// The key names/values when a credential has multiple pieces of credential information.
	/// This will be adapter-specific.
	/// NOTE: Not used yet.
	MultiKeys(HashMap<String, String>),
}

/// Constructors
impl AuthData {
	/// Create a new `AuthData` from an environment variable name.
	pub fn from_env(env_name: impl Into<String>) -> Self {
		AuthData::FromEnv(env_name.into())
	}

	/// Create a new `AuthData` from a single value.
	pub fn from_single(value: impl Into<String>) -> Self {
		AuthData::Key(value.into())
	}

	/// Create a new `AuthData` from multiple values.
	pub fn from_multi(data: HashMap<String, String>) -> Self {
		AuthData::MultiKeys(data)
	}
}

/// Getters
impl AuthData {
	/// Get the single value from the `AuthData`.
	pub fn single_value(&self) -> Result<String> {
		match self {
			AuthData::FromEnv(env_name) => {
				// Get value from the environment name.
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

// Implement Debug to redact sensitive information.
impl std::fmt::Debug for AuthData {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			// NOTE: Here we also redact for `FromEnv` in case the developer confuses this with a key.
			AuthData::FromEnv(_env_name) => write!(f, "AuthData::FromEnv(REDACTED)"),
			AuthData::Key(_) => write!(f, "AuthData::Single(REDACTED)"),
			AuthData::MultiKeys(_) => write!(f, "AuthData::Multi(REDACTED)"),
		}
	}
}

// endregion: --- AuthData Std Impls
