use crate::adapter::AdapterKind;
use std::ops::Deref;
use std::sync::Arc;

// region:    --- ModelInfo

/// Hold the adapter_kind and model_name in a efficient clonable way
/// Note: For now,
#[derive(Clone, Debug)]
pub struct ModelInfo {
	pub adapter_kind: AdapterKind,
	pub model_name: ModelName,
}

impl ModelInfo {
	pub fn new(adapter_kind: AdapterKind, model_name: impl Into<ModelName>) -> Self {
		Self {
			adapter_kind,
			model_name: model_name.into(),
		}
	}
}

impl<T> From<(AdapterKind, T)> for ModelInfo
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

// endregion: --- ModelInfo

// region:    --- ModelName

#[derive(Clone, Debug)]
pub struct ModelName(Arc<str>);

impl std::fmt::Display for ModelName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

// -- Froms

impl From<ModelName> for String {
	fn from(model_name: ModelName) -> Self {
		model_name.0.to_string()
	}
}

// NOTE: Below we avoid the `T: Into<String>` blanket implementation because
//       it would prevent us to have the `From<ModelName> for String` as `ModelName`
//       also implement `T: Into<String>` from it's deref to `&str`

impl From<String> for ModelName {
	fn from(s: String) -> Self {
		Self(Arc::from(s))
	}
}

impl From<&String> for ModelName {
	fn from(s: &String) -> Self {
		Self(Arc::from(s.as_str()))
	}
}

impl From<&str> for ModelName {
	fn from(s: &str) -> Self {
		Self(Arc::from(s))
	}
}

/// Deref as `&str`
impl Deref for ModelName {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

// endregion: --- ModelName
