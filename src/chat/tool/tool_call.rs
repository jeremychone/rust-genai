use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The tool call function name and arguments send back by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
	pub call_id: String,
	pub fn_name: String,
	pub fn_arguments: Value,
}
