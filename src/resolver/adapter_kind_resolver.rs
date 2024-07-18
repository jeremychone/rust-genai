//! An AdapterKindResolver is responsible for resolving the AdapterKind based on the provided model.
//! It uses a SyncAdapterKindResolverFn trait to accomplish this.

use crate::adapter::AdapterKind;
use crate::Result;
use std::sync::Arc;

#[derive(Clone)]
pub struct AdapterKindResolver {
	inner: Arc<dyn SyncAdapterKindResolverFn>,
}

impl std::fmt::Debug for AdapterKindResolver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "AdapterKindResolver")
	}
}

impl AdapterKindResolver {
	pub fn from_sync_resolver(resolver_fn: impl IntoSyncAdapterKindResolverFn) -> Self {
		AdapterKindResolver {
			inner: resolver_fn.into_sync_resolver_fn(),
		}
	}

	pub fn resolve(&self, model: &str) -> Result<Option<AdapterKind>> {
		self.inner.exec_sync_resolver_fn(model)
	}
}

// Define a trait for the SyncAdapterKindResolverFn
pub trait SyncAdapterKindResolverFn: Send + Sync {
	fn exec_sync_resolver_fn(&self, model: &str) -> Result<Option<AdapterKind>>;
}

// Define a trait for types that can be converted into Arc<dyn SyncAdapterKindResolverFn>
pub trait IntoSyncAdapterKindResolverFn {
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAdapterKindResolverFn>;
}

// Implement IntoSyncAdapterKindResolverFn for Arc<dyn SyncAdapterKindResolverFn>
impl IntoSyncAdapterKindResolverFn for Arc<dyn SyncAdapterKindResolverFn> {
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAdapterKindResolverFn> {
		self
	}
}

// Implement IntoSyncAdapterKindResolverFn for closures
impl<F> IntoSyncAdapterKindResolverFn for F
where
	F: Fn(&str) -> Result<Option<AdapterKind>> + Send + Sync + 'static,
{
	fn into_sync_resolver_fn(self) -> Arc<dyn SyncAdapterKindResolverFn> {
		Arc::new(self)
	}
}

// Implement SyncAdapterKindResolverFn for closures
impl<F> SyncAdapterKindResolverFn for F
where
	F: Fn(&str) -> Result<Option<AdapterKind>> + Send + Sync,
{
	fn exec_sync_resolver_fn(&self, model: &str) -> Result<Option<AdapterKind>> {
		self(model)
	}
}
