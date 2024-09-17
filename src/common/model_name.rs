use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The model name, which is just an `Arc<str>` wrapper (simple and relatively efficient to clone)
#[derive(Clone, Debug, Serialize, Deserialize)]
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
//       it would prevent us from having the `From<ModelName> for String` as `ModelName`
//       also implements `T: Into<String>` from its deref to `&str`

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
