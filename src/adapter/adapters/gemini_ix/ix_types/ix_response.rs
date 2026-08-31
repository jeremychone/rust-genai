use super::{IxStep, IxUsage};
use crate::chat::StopReason;
use serde::Deserialize;
use serde_json::Value;

/// The `Interaction` resource.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Interaction>
#[derive(Debug, Clone, Deserialize)]
pub struct IxInteraction {
	/// Unique identifier for the interaction. Feed back as `previous_interaction_id`.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub id: Option<String>,

	/// `completed | in_progress | requires_action | failed | cancelled | incomplete |
	/// budget_exceeded | queued`
	#[serde(default)]
	pub status: Option<String>,

	/// Provider-reported model name. Absent on some SSE frames.
	#[serde(default)]
	pub model: Option<String>,

	/// The interaction timeline. Absent on `interaction.created`.
	#[serde(default)]
	pub steps: Vec<IxStep>,

	#[serde(default)]
	pub usage: Option<IxUsage>,

	/// Diagnostic faults recorded on the interaction.
	#[serde(default)]
	pub errors: Vec<Value>,
}

impl IxInteraction {
	/// Renders `errors` into a single human-readable message, if any were recorded.
	pub fn error_message(&self) -> Option<String> {
		if self.errors.is_empty() {
			return None;
		}
		let messages: Vec<String> = self
			.errors
			.iter()
			.map(|error| {
				let code = error.get("code").and_then(Value::as_str).unwrap_or("unknown");
				let message = error.get("message").and_then(Value::as_str).unwrap_or_default();
				format!("[{code}] {message}")
			})
			.collect();
		Some(messages.join("; "))
	}
}

/// Maps an interaction `status` onto the normalized `StopReason`.
pub fn ix_status_to_stop_reason(status: Option<String>) -> Option<StopReason> {
	let status = status?;
	let stop_reason = match status.as_str() {
		// The model is waiting on a client-side `function_result`.
		"requires_action" => StopReason::ToolCall(status),
		// Halted on the token budget — the same class of truncation as `incomplete`.
		"budget_exceeded" => StopReason::MaxTokens(status),
		_ => StopReason::from(status),
	};
	Some(stop_reason)
}
