use derive_more::From;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use value_ext::JsonValueExt as _;

use crate::{ModelIden, adapter::AdapterKind};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct CustomPart {
	pub model_iden: ModelIden,
	pub data: Value,
}

impl CustomPart {
	pub fn adapter_kind(&self) -> AdapterKind {
		self.model_iden.adapter_kind
	}
	pub fn data(&self) -> &Value {
		&self.data
	}

	/// Returns the "type" field from the data, if it exists and is a string.
	pub fn typ(&self) -> Option<&str> {
		self.data.x_get_str("type").ok()
	}
}
