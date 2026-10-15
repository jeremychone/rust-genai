use crate::chat::{CompletionTokensDetails, PromptTokensDetails, Usage};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

/// Token usage as reported by the Interactions API.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Interaction> (`usage`)
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct IxUsage {
	pub total_input_tokens: Option<i32>,

	/// Input tokens split by modality. The audio share feeds `PromptTokensDetails::audio_tokens`.
	#[serde(default)]
	pub input_tokens_by_modality: Vec<IxModalityTokens>,

	pub total_cached_tokens: Option<i32>,

	pub total_output_tokens: Option<i32>,

	/// Output tokens split by modality. The audio share feeds `CompletionTokensDetails::audio_tokens`.
	#[serde(default)]
	pub output_tokens_by_modality: Vec<IxModalityTokens>,

	pub total_thought_tokens: Option<i32>,

	pub total_tool_use_tokens: Option<i32>,

	/// Server-side tool input (search results, fetched pages) split by modality. Counted as prompt input.
	#[serde(default)]
	pub tool_use_tokens_by_modality: Vec<IxModalityTokens>,

	pub total_tokens: Option<i32>,
}

/// Token count for one modality.
///
/// DOC: <https://ai.google.dev/api/interactions#Resource:Usage> (`*_tokens_by_modality`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IxModalityTokens {
	/// `text | image | audio | video | document`. Kept as a string so a new modality cannot break parsing.
	pub modality: String,

	#[serde(default)]
	pub tokens: i32,
}

impl From<IxUsage> for Usage {
	fn from(ix_usage: IxUsage) -> Self {
		let IxUsage {
			total_input_tokens,
			input_tokens_by_modality,
			total_cached_tokens,
			total_output_tokens,
			output_tokens_by_modality,
			total_thought_tokens,
			total_tool_use_tokens,
			tool_use_tokens_by_modality,
			total_tokens,
		} = ix_usage;

		let prompt_tokens = match (total_input_tokens, total_tool_use_tokens) {
			(Some(input), Some(tool_use)) => Some(input + tool_use),
			(None, Some(tool_use)) => Some(tool_use),
			(input, None) => input,
		};

		let cached_tokens = total_cached_tokens.filter(|tokens| *tokens > 0);
		let prompt_audio_tokens = modality_tokens(&[&input_tokens_by_modality, &tool_use_tokens_by_modality], "audio");
		let prompt_tokens_details =
			(cached_tokens.is_some() || prompt_audio_tokens.is_some()).then_some(PromptTokensDetails {
				cache_creation_tokens: None,
				cache_creation_details: None,
				cached_tokens,
				audio_tokens: prompt_audio_tokens,
			});

		let reasoning_tokens = total_thought_tokens.filter(|tokens| *tokens > 0);
		let completion_audio_tokens = modality_tokens(&[&output_tokens_by_modality], "audio");

		// As with `generateContent`, `total_thought_tokens` is NOT included in `total_output_tokens`.
		let completion_tokens = match (total_output_tokens, reasoning_tokens) {
			(Some(output), Some(thought)) => Some(output + thought),
			(None, Some(thought)) => Some(thought),
			(output, None) => output,
		};
		let completion_tokens_details =
			(reasoning_tokens.is_some() || completion_audio_tokens.is_some()).then_some(CompletionTokensDetails {
				accepted_prediction_tokens: None,
				rejected_prediction_tokens: None,
				reasoning_tokens,
				audio_tokens: completion_audio_tokens,
			});

		// The reported total wins; the computed sum is only a fallback when both halves are known.
		let total_tokens = total_tokens.or_else(|| {
			prompt_tokens
				.zip(completion_tokens)
				.map(|(prompt, completion)| prompt + completion)
		});

		Usage {
			prompt_tokens,
			prompt_tokens_details,
			completion_tokens,
			completion_tokens_details,
			total_tokens,
		}
	}
}

// region:    --- Support

/// Sums the tokens of one modality across the given breakdowns. `None` when absent or zero.
fn modality_tokens(breakdowns: &[&[IxModalityTokens]], modality: &str) -> Option<i32> {
	let total: i32 = breakdowns
		.iter()
		.flat_map(|entries| entries.iter())
		.filter(|entry| entry.modality == modality)
		.map(|entry| entry.tokens)
		.sum();
	(total > 0).then_some(total)
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "ix_usage_tests.rs"]
mod tests;

// endregion: --- Tests
