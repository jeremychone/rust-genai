use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// The normalized LLM input/output token usage (based on the OpenAI API).
///
/// **NOTE:** The serialization of Usage will treat '0' as None. This is the most consistent way to handle it across models.
///           OpenAI uses 0 in many places even when it is not relevant to the request.
///
/// > **NOTE:** The `prompt_tokens` and `completion_tokens` are normalized to represent the total tokens for input and output for all models/providers.
/// > And, `prompt_tokens_details` and `completion_tokens_details` may provide more detailed information about the composition of these tokens.
/// >
/// > For example: For Gemini, the `thoughts_token_count` (~reasoning_tokens) is not included
/// > in the root `candidatesTokenCount` (~completion_tokens).
/// > Therefore, when `thoughts_token_count` genail will do the necessary computation
/// > to normalize it in the "OpenAI API Way,"
/// > meaning `completion_tokens` represents the total of completion tokens (`candidatesTokenCount + thoughts_token_count`),
/// > and the `completion_tokens_details.reasoning_tokens` will have the `thoughts_token_count`
///
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
	/// The input tokens (replaces input_tokens)
	pub prompt_tokens: Option<i32>,
	pub prompt_tokens_details: Option<PromptTokensDetails>,

	/// The completions/output tokens
	pub completion_tokens: Option<i32>,
	pub completion_tokens_details: Option<CompletionTokensDetails>,

	/// The total number of tokens if returned by the API call.
	/// This will either be the total_tokens if returned,
	/// or the sum of prompt/completion including the cache and cache creation tokens.
	pub total_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
	/// Anthropic only for now (this maps to Anthropic 'cache_creation_input_tokens')
	/// This is the token that are not cache yet, but were use to create the cache
	/// Anthropic has a little surcharge for those (25%), but then, we get 90% on next call on the cached_tokens
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cache_creation_tokens: Option<i32>,
	/// For Anthropic this will be the `cache_read_input_tokens`
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cached_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub audio_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub accepted_prediction_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub rejected_prediction_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub reasoning_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub audio_tokens: Option<i32>,
}
