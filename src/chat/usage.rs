use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

#[deprecated(note = "MetaUsage has been renamed to Usage. Please use Usage instead.")]
pub type MetaUsage = Usage;

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
	/// This will either be the total_tokens if returned, or the sum of prompt/completion if not specified in the response.
	pub total_tokens: Option<i32>,

	// -- Deprecated
	/// The number of input tokens if returned by the API call.
	#[deprecated(note = "Use prompt_tokens (for now it is a clone, but later will be removed)")]
	#[serde(skip)]
	pub input_tokens: Option<i32>,

	/// The number of output tokens if returned by the API call.
	#[deprecated(note = "Use prompt_tokens (for now it is a clone, but later will be removed)")]
	#[serde(skip)]
	pub output_tokens: Option<i32>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
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
