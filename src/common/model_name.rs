use std::ops::Deref;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// The model name, which is just an `Arc<str>` wrapper (simple and relatively efficient to clone)
#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct ModelName(Arc<str>);

impl ModelName {
	pub fn namespace_is(&self, namespace: &str) -> bool {
		self.namespace() == Some(namespace)
	}

	pub fn namespace(&self) -> Option<&str> {
		self.namespace_and_name().0
	}

	/// Returns `(namespace, name)`
	pub fn namespace_and_name(&self) -> (Option<&str>, &str) {
		Self::split_as_namespace_and_name(&self.0)
	}

	/// e.g.:
	/// `openai::gpt4.1` → (Some("openai"), "gpt4.1")
	/// `gpt4.1`         → (None, "gpt4.1")
	pub(crate) fn split_as_namespace_and_name(model: &str) -> (Option<&str>, &str) {
		if let Some(ns_idx) = model.find("::") {
			let ns: &str = &model[..ns_idx];
			let name: &str = &model[(ns_idx + 2)..];
			(Some(ns), name)
		} else {
			(None, model)
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
