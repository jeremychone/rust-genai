use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

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
	pub cache_creation_tokens: Option<i32>,
	/// For Anthropic this will be the `cache_read_input_tokens`
	pub cached_tokens: Option<i32>,
	pub audio_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
	pub accepted_prediction_tokens: Option<i32>,
	pub rejected_prediction_tokens: Option<i32>,
	pub reasoning_tokens: Option<i32>,
	pub audio_tokens: Option<i32>,
}
