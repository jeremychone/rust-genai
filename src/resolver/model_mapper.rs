use crate::resolver::Result;
use crate::ModelIden;
use std::sync::Arc;

// region:    --- ModelMapper

/// An ModelMapper for mapping a resolved `ModelIden` (i.e. AdapterKind + ModelName) to another one.
/// It must return a `ModelIden` or an appropriate
#[derive(Debug, Clone)]
pub enum ModelMapper {
	MapperFn(Arc<Box<dyn ModelMapperFn>>),
}

impl ModelMapper {
	pub fn from_mapper_fn(mapper_fn: impl IntoModelMapperFn) -> Self {
		ModelMapper::MapperFn(mapper_fn.into_mapper_fn())
	}
}

impl ModelMapper {
	pub(crate) fn map_model(&self, model_iden: ModelIden) -> Result<ModelIden> {
		match self {
			ModelMapper::MapperFn(mapper_fn) => {
				// Clone the Arc to get a new reference to the Box, then call exec_fn
				mapper_fn.clone().exec_fn(model_iden)
			}
		}
	}
}

// endregion: --- ModelMapper

// region:    --- ModelMapperFn

// Define the trait for an auth resolver function
pub trait ModelMapperFn: Send + Sync {
	fn exec_fn(&self, model_iden: ModelIden) -> Result<ModelIden>;
	fn clone_box(&self) -> Box<dyn ModelMapperFn>;
}

// Implement ModelMapperFn for any `FnOnce`
impl<F> ModelMapperFn for F
where
	F: FnOnce(ModelIden) -> Result<ModelIden> + Send + Sync + Clone + 'static,
{
	fn exec_fn(&self, model_iden: ModelIden) -> Result<ModelIden> {
		(self.clone())(model_iden)
	}

	fn clone_box(&self) -> Box<dyn ModelMapperFn> {
		Box::new(self.clone())
	}
}

// Implement Clone for Box<dyn ModelMapperFn>
impl Clone for Box<dyn ModelMapperFn> {
	fn clone(&self) -> Box<dyn ModelMapperFn> {
		self.clone_box()
	}
}

impl std::fmt::Debug for dyn ModelMapperFn {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ModelMapperFn")
	}
}

// endregion: --- ModelMapperFn

// region:    --- IntoModelMapperFn

pub trait IntoModelMapperFn {
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>>;
}

impl IntoModelMapperFn for Arc<Box<dyn ModelMapperFn>> {
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>> {
		self
	}
}

// Implement IntoModelMapperFn for closures
impl<F> IntoModelMapperFn for F
where
	F: FnOnce(ModelIden) -> Result<ModelIden> + Send + Sync + Clone + 'static,
{
	fn into_mapper_fn(self) -> Arc<Box<dyn ModelMapperFn>> {
		Arc::new(Box::new(self))
	}
}

// endregion: --- IntoModelMapperFn
