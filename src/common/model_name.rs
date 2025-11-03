use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The model name, which is just an `Arc<str>` wrapper (simple and relatively efficient to clone)
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ModelName(Arc<str>);

/// Utilities
impl ModelName {
	/// Calling the `model_name_and_namespace`
	pub(crate) fn as_model_name_and_namespace(&self) -> (&str, Option<&str>) {
		Self::model_name_and_namespace(&self.0)
	}

	/// e.g., `openai::gpt4.1` ("gpt4.1", Some("openai"))
	///       `gpt4.1` ("gpt4.1", None)
	pub(crate) fn model_name_and_namespace(model: &str) -> (&str, Option<&str>) {
		if let Some(ns_idx) = model.find("::") {
			let ns: &str = &model[..ns_idx];
			let name: &str = &model[(ns_idx + 2)..];
			// TODO: assess what to do when name or ns is empty
			(name, Some(ns))
		} else {
			(model, None)
		}
	}
}

// region:    --- Froms

impl From<ModelName> for String {
	fn from(model_name: ModelName) -> Self {
		model_name.0.to_string()
	}
}

// NOTE: Below we avoid the `T: Into<String>` blanket implementation because
//       it would prevent us from having the `From<ModelName> for String` implementation since `ModelName`
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

// endregion: --- Froms

// region:    --- EQ

// PartialEq implementations for various string types
impl PartialEq<str> for ModelName {
	fn eq(&self, other: &str) -> bool {
		&*self.0 == other
	}
}

impl PartialEq<&str> for ModelName {
	fn eq(&self, other: &&str) -> bool {
		&*self.0 == *other
	}
}

impl PartialEq<String> for ModelName {
	fn eq(&self, other: &String) -> bool {
		&*self.0 == other
	}
}

// Symmetric implementations (allow "string" == model_name)
impl PartialEq<ModelName> for str {
	fn eq(&self, other: &ModelName) -> bool {
		self == &*other.0
	}
}

impl PartialEq<ModelName> for &str {
	fn eq(&self, other: &ModelName) -> bool {
		*self == &*other.0
	}
}

impl PartialEq<ModelName> for String {
	fn eq(&self, other: &ModelName) -> bool {
		self == &*other.0
	}
}

// endregion: --- EQ

// TODO: replace with derive_more Display
impl std::fmt::Display for ModelName {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}
