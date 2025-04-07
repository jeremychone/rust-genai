//! An `AuthResolver` is responsible for returning the `AuthData` (typically containing the `api_key`).
//! It can take the following forms:
//! - Configured with a custom environment name,
//! - Contains a fixed auth value,
//! - Contains an `AuthResolverFn` trait object or closure that will be called to return the `AuthData`.
//!
//! Note: `AuthData` is typically a single value but can be multiple for future adapters (e.g., AWS Bedrock).

use crate::resolver::{AuthData, Error, Result};
use crate::ModelIden;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// region:    --- AuthResolver

/// Holder for the `AuthResolver` function.
/// NOTE: Eventually, we might also have a `ResolveAsyncFn`, hence the enum.
#[derive(Debug, Clone)]
pub enum AuthResolver {
	/// The `AuthResolverFn` trait object.
	ResolverFn(Arc<Box<dyn AuthResolverFn>>),
	///
	ResolverAsyncFn(Arc<Box<dyn AuthResolverAsyncFn>>),
}

impl AuthResolver {
	/// Create a new `AuthResolver` from a resolver function.
	pub fn from_resolver_fn(resolver_fn: impl IntoAuthResolverFn) -> Self {
		AuthResolver::ResolverFn(resolver_fn.into_resolver_fn())
	}
	pub fn from_resolver_async_fn(resolver_fn: impl IntoAuthResolverAsyncFn) -> Self {
		AuthResolver::ResolverAsyncFn(resolver_fn.into_resolver_fn())
	}
}

impl AuthResolver {
	#[deprecated(note = "use resolve_async")]
	pub(crate) fn resolve(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		match self {
			AuthResolver::ResolverFn(resolver_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn.
				resolver_fn.exec_fn(model_iden)
			}
			AuthResolver::ResolverAsyncFn(_) => Err(Error::UnsupportedUsageOfAsyncResolver(
				"Use non-async resolver or use AuthResolver::resolve_async".to_string(),
			)),
		}
	}
	pub(crate) async fn resolve_async(&self, model_iden: ModelIden) -> Result<Option<AuthData>> {
		match self {
			AuthResolver::ResolverFn(resolver_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn.
				resolver_fn.clone().exec_fn(model_iden)
			}
			AuthResolver::ResolverAsyncFn(resolver_fn) => resolver_fn.clone().exec_fn(model_iden).await,
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

pub trait AuthResolverAsyncFn: Send + Sync {
	/// Execute the `AuthResolverFn` to get the `AuthData`.
	fn exec_fn(&self, model_iden: ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>>>>;

	///	Clone the trait object.
	fn clone_box(&self) -> Box<dyn AuthResolverAsyncFn>;
}

impl<F: Send> AuthResolverAsyncFn for F
where
	F: AsyncFnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Pin<Box<dyn Future<Output = Result<Option<AuthData>>>>> {
		let resolver = self.clone();
		Box::pin(async { resolver(model_iden).await })
	}

	fn clone_box(&self) -> Box<dyn AuthResolverAsyncFn> {
		Box::new(self.clone())
	}
}

impl std::fmt::Debug for dyn AuthResolverAsyncFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AuthResolverAsyncFn")
	}
}

pub trait IntoAuthResolverAsyncFn {
	/// Convert the argument into an `AuthResolverFn` trait object.
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverAsyncFn>>;
}

// Implement `IntoAuthResolverFn` for closures.
impl<F> IntoAuthResolverAsyncFn for F
where
	F: AsyncFnOnce(ModelIden) -> Result<Option<AuthData>> + Send + Sync + Clone + 'static,
{
	fn into_resolver_fn(self) -> Arc<Box<dyn AuthResolverAsyncFn>> {
		Arc::new(Box::new(self))
	}
}
