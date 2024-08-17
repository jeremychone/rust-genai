use crate::adapter::AdapterKind;
use crate::ModelName;

/// Hold the adapter_kind and model_name in a efficient clonable way
/// Note: For now,
#[derive(Clone, Debug)]
pub struct ModelIden {
	pub adapter_kind: AdapterKind,
	pub model_name: ModelName,
}

impl ModelIden {
	pub fn new(adapter_kind: AdapterKind, model_name: impl Into<ModelName>) -> Self {
		Self {
			adapter_kind,
			model_name: model_name.into(),
		}
	}
}

impl<T> From<(AdapterKind, T)> for ModelIden
where
	T: Into<ModelName>,
{
	fn from((adapter_kind, model_name): (AdapterKind, T)) -> Self {
		Self {
			adapter_kind,
			model_name: model_name.into(),
		}
	}
}
