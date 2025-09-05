use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The tool call function name and arguments sent back by the LLM.
/// Represents a single function/tool invocation emitted by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
	/// Stable identifier for this tool call (used to correlate tool responses).
	pub call_id: String,

	/// Name of the function to invoke.
	pub fn_name: String,

	/// JSON arguments payload as provided by the model.
	/// Kept as `serde_json::Value` so callers can deserialize into their own types.
	pub fn_arguments: Value,
}
