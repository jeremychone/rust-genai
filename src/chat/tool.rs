use serde_json::Value;

#[allow(unused)] // Not used yet
pub struct Tool {
	fn_name: String,
	fn_description: String,
	params: Value,
}
