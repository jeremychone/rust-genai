//! A `ServiceTargetResolver` is responsible for returning the `ServiceTarget`.
//! It allows users to customize/override the service target properties.
//! 
//! It can take the following forms:
//! - Contains a fixed service target value,
//! - Contains a `ServiceTargetResolverFn` trait object or closure that will be called to return the `ServiceTarget`.

use crate::ServiceTarget;
use crate::resolver::Result;
use std::sync::Arc;

// region:    --- ServiceTargetResolver

/// Holder for the `ServiceTargetResolver` function.
#[derive(Debug, Clone)]
pub enum ServiceTargetResolver {
    /// The `ServiceTargetResolverFn` trait object.
    ResolverFn(Arc<Box<dyn ServiceTargetResolverFn>>),
}

impl ServiceTargetResolver {
    /// Create a new `ServiceTargetResolver` from a resolver function.
    pub fn from_resolver_fn(resolver_fn: impl IntoServiceTargetResolverFn) -> Self {
        ServiceTargetResolver::ResolverFn(resolver_fn.into_resolver_fn())
    }
}

impl ServiceTargetResolver {
    pub(crate) fn resolve(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
        match self {
            ServiceTargetResolver::ResolverFn(resolver_fn) => {
                resolver_fn.clone().exec_fn(service_target)
            }
        }
    }
}

// endregion: --- ServiceTargetResolver

// region:    --- ServiceTargetResolverFn

/// The `ServiceTargetResolverFn` trait object.
pub trait ServiceTargetResolverFn: Send + Sync {
    /// Execute the `ServiceTargetResolverFn` to get the `ServiceTarget`.
    fn exec_fn(&self, service_target: ServiceTarget) -> Result<ServiceTarget>;

    /// Clone the trait object.
    fn clone_box(&self) -> Box<dyn ServiceTargetResolverFn>;
}

/// `ServiceTargetResolverFn` blanket implementation for any function that matches the resolver function signature.
impl<F> ServiceTargetResolverFn for F
where
    F: FnOnce(ServiceTarget) -> Result<ServiceTarget> + Send + Sync + Clone + 'static,
{
    fn exec_fn(&self, service_target: ServiceTarget) -> Result<ServiceTarget> {
        (self.clone())(service_target)
    }

    fn clone_box(&self) -> Box<dyn ServiceTargetResolverFn> {
        Box::new(self.clone())
    }
}

// Implement Clone for Box<dyn ServiceTargetResolverFn>
impl Clone for Box<dyn ServiceTargetResolverFn> {
    fn clone(&self) -> Box<dyn ServiceTargetResolverFn> {
        self.clone_box()
    }
}

impl std::fmt::Debug for dyn ServiceTargetResolverFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ServiceTargetResolverFn")
    }
}

// endregion: --- ServiceTargetResolverFn

// region:    --- IntoServiceTargetResolverFn

/// Custom and convenient trait used in the `ServiceTargetResolver::from_resolver_fn` argument.
pub trait IntoServiceTargetResolverFn {
    /// Convert the argument into a `ServiceTargetResolverFn` trait object.
    fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>>;
}

impl IntoServiceTargetResolverFn for Arc<Box<dyn ServiceTargetResolverFn>> {
    fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>> {
        self
    }
}

// Implement `IntoServiceTargetResolverFn` for closures.
impl<F> IntoServiceTargetResolverFn for F
where
    F: FnOnce(ServiceTarget) -> Result<ServiceTarget> + Send + Sync + Clone + 'static,
{
    fn into_resolver_fn(self) -> Arc<Box<dyn ServiceTargetResolverFn>> {
        Arc::new(Box::new(self))
    }
}

// endregion: --- IntoServiceTargetResolverFn
