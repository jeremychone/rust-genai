use serde::{Deserialize, Serialize};

/// One provider-issued thinking block whose text and integrity signature must
/// remain bound and ordered when the assistant turn is replayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThinkingBlock {
	pub thinking: String,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub signature: Option<String>,
}

impl ThinkingBlock {
	pub fn new(thinking: impl Into<String>, signature: Option<String>) -> Self {
		Self {
			thinking: thinking.into(),
			signature,
		}
	}

	pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
		self.signature = Some(signature.into());
		self
	}
}
