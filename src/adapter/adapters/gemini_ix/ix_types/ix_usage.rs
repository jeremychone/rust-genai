use crate::chat::{CompletionTokensDetails, PromptTokensDetails, Usage};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Token usage as reported by the Interactions API.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Interaction> (`usage`)
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct IxUsage {
	pub total_input_tokens: Option<i32>,

	pub total_cached_tokens: Option<i32>,

	pub total_output_tokens: Option<i32>,

	pub total_thought_tokens: Option<i32>,

	pub total_tool_use_tokens: Option<i32>,

	pub total_tokens: Option<i32>,
}

impl From<IxUsage> for Usage {
	fn from(ix_usage: IxUsage) -> Self {
		let IxUsage {
			total_input_tokens,
			total_cached_tokens,
			total_output_tokens,
			total_thought_tokens,
			total_tool_use_tokens,
			total_tokens,
		} = ix_usage;

		let prompt_tokens = match (total_input_tokens, total_tool_use_tokens) {
			(Some(input), Some(tool_use)) => Some(input + tool_use),
			(None, Some(tool_use)) => Some(tool_use),
			(input, None) => input,
		};

		let prompt_tokens_details =
			total_cached_tokens
				.filter(|tokens| *tokens > 0)
				.map(|cached_tokens| PromptTokensDetails {
					cache_creation_tokens: None,
					cache_creation_details: None,
					cached_tokens: Some(cached_tokens),
					audio_tokens: None,
				});

		// As with `generateContent`, `total_thought_tokens` is NOT included in `total_output_tokens`.
		let (completion_tokens, completion_tokens_details) = match (total_output_tokens, total_thought_tokens) {
			(Some(output), Some(thought)) if thought > 0 => (
				Some(output + thought),
				Some(CompletionTokensDetails {
					accepted_prediction_tokens: None,
					rejected_prediction_tokens: None,
					reasoning_tokens: Some(thought),
					audio_tokens: None,
				}),
			),
			(None, Some(thought)) if thought > 0 => (
				None,
				Some(CompletionTokensDetails {
					accepted_prediction_tokens: None,
					rejected_prediction_tokens: None,
					reasoning_tokens: Some(thought),
					audio_tokens: None,
				}),
			),
			(output, _) => (output, None),
		};

		Usage {
			prompt_tokens,
			prompt_tokens_details,
			completion_tokens,
			completion_tokens_details,
			total_tokens,
		}
	}
}
