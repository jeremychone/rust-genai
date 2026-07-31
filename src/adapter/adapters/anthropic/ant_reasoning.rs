use super::ant_model::AnthropicModelCapabilities;
use crate::{Result, chat::ReasoningEffort};
use serde_json::{Map, Value, json};
use value_ext::JsonValueExt;

// region:    --- Reasoning Support

pub(super) const REASONING_LOW: u32 = 1024;
pub(super) const REASONING_MEDIUM: u32 = 8000;
pub(super) const REASONING_HIGH: u32 = 24000;

pub(super) fn insert_anthropic_reasoning(
	payload: &mut Value,
	output_config: &mut Map<String, Value>,
	capabilities: &AnthropicModelCapabilities,
	effort: &ReasoningEffort,
	capture_reasoning_content: bool,
) -> Result<()> {
	let mut budget: Option<u32> = None;
	let support_effort = capabilities.supports_effort;
	let support_reasoning_max = capabilities.supports_max_effort;
	let support_adaptive = capabilities.supports_adaptive_thinking;
	let support_xhigh = capabilities.supports_xhigh_effort;

	// `Zero` means "no reasoning": emit no `output_config.effort` and no adaptive/legacy
	// thinking payload. Models where thinking is on by default need an explicit opt-out;
	// for all others, omitting the `thinking` field already means "off".
	if matches!(effort, ReasoningEffort::Zero) {
		if capabilities.thinking_enabled_by_default {
			payload.x_insert("thinking", json!({"type": "disabled"}))?;
		}
		return Ok(());
	}

	// Models that support effort use it as the primary reasoning control.
	if support_effort {
		let effort = match effort {
			ReasoningEffort::Minimal => "low",
			ReasoningEffort::Low => "low",
			ReasoningEffort::Medium => "medium",
			ReasoningEffort::High => "high",
			ReasoningEffort::XHigh if support_xhigh => "xhigh",
			ReasoningEffort::Max if support_reasoning_max => "max",
			ReasoningEffort::XHigh => "high",
			ReasoningEffort::Max => "high",
			// Preserve explicit budget tokens for the adaptive thinking payload below.
			ReasoningEffort::Budget(val) => {
				budget = Some(*val);
				""
			}
			// Handled by the early return above; kept for exhaustiveness.
			ReasoningEffort::Zero => "",
		};

		// Emit the effort into the shared output_config map when present.
		if !effort.is_empty() {
			output_config.insert("effort".to_string(), json!(effort));
		}
	}

	// Adaptive-thinking models use a `thinking` object, optionally with budget tokens.
	if support_adaptive {
		let mut thinking = match budget {
			Some(budget) => json!({
						"type": "adaptive",
						"budget_tokens": budget // if None, should be ok.
			}),
			None => json!({
				"type": "adaptive"}),
		};
		if capture_reasoning_content {
			thinking.x_insert("display", "summarized")?;
		}

		// Let the model choose adaptive thinking behavior, honoring an explicit budget when set.
		payload.x_insert("thinking", thinking)?;
	}

	// Older models still use the legacy `thinking.enabled + budget_tokens` shape.
	if !support_effort || matches!(effort, ReasoningEffort::Budget(_)) && capabilities.supports_legacy_budget_thinking {
		let thinking_budget = match effort {
			ReasoningEffort::Zero => None,
			ReasoningEffort::Budget(budget) => Some(*budget),
			ReasoningEffort::Low | ReasoningEffort::Minimal => Some(REASONING_LOW),
			ReasoningEffort::Medium => Some(REASONING_MEDIUM),
			ReasoningEffort::High | ReasoningEffort::Max | ReasoningEffort::XHigh => Some(REASONING_HIGH),
		};

		if let Some(thinking_budget) = thinking_budget {
			payload.x_insert(
				"thinking",
				json!({
					"type": "enabled",
					"budget_tokens": thinking_budget
				}),
			)?;
		}
	}

	Ok(())
}

// region:    --- Tests

#[cfg(test)]
#[path = "ant_reasoning_tests.rs"]
mod tests;

// endregion: --- Tests
