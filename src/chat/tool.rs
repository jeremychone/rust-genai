use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(unused)] // Not used yet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
	fn_name: String,
	fn_description: String,
	params: Value,
}
