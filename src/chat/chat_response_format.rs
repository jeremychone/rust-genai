use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub enum ChatResponseFormat {
	JsonMode,
	#[from]
	StructuredJson(StructuredJson),
}

#[derive(Debug, Clone, From, Serialize, Deserialize)]
pub struct StructuredJson {
	pub name: String,
	pub description: Option<String>,
	pub schema: Value,
}

/// Constructors
impl StructuredJson {
	pub fn new(name: impl Into<String>, schema: impl Into<Value>) -> Self {
		Self {
			name: name.into(),
			description: None,
			schema: schema.into(),
		}
	}
}

/// Setters
impl StructuredJson {
	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}
}
