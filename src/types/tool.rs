use serde_json::Value;

#[allow(unused)] // For early development.
pub struct Tool {
	fn_name: String,
	fn_description: String,
	params: Value,
}
