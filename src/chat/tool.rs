use serde::{Deserialize, Serialize};
use serde_json::Value;

/// NOT USED FOR NOW
/// > For later, it will be used for function calling
/// > It will probably use the JsonSpec type we had in the response format,
/// > or have a `From<JsonSpec>` implementation.
#[allow(unused)] // Not used yet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
	fn_name: String,
	fn_description: String,
	params: Value,
}