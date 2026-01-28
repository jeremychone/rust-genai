use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};

/// Normalized token usage across providers (OpenAI-compatible).
///
/// - Deserialization treats 0 as None for cross-provider consistency. OpenAI often returns 0 for non-applicable counters.
///
/// - `prompt_tokens` and `completion_tokens` are the total input/output tokens. The corresponding `*_details` carry provider-specific breakdowns.
///
/// - Gemini: `candidatesTokenCount` excludes "thoughts" (reasoning) tokens. We normalize:
///   `completion_tokens = candidatesTokenCount + thoughts_token_count`, and
///   `completion_tokens_details.reasoning_tokens = thoughts_token_count`.
#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
	/// Total input tokens (formerly `input_tokens`).
	pub prompt_tokens: Option<i32>,
	pub prompt_tokens_details: Option<PromptTokensDetails>,

	/// Total output (completion) tokens.
	pub completion_tokens: Option<i32>,
	pub completion_tokens_details: Option<CompletionTokensDetails>,

	/// Total tokens as reported by the API, or computed as prompt + completion
	/// (including cache read/creation tokens when applicable).
	pub total_tokens: Option<i32>,
}

impl Usage {
	/// Remove detail objects that contain only `None` fields.
	pub fn compact_details(&mut self) {
		if self.prompt_tokens_details.as_ref().is_some_and(|d| d.is_empty()) {
			self.prompt_tokens_details = None;
		}

		if self.completion_tokens_details.as_ref().is_some_and(|d| d.is_empty()) {
			self.completion_tokens_details = None;
		}
	}
}

#[serde_as]
#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
	/// Anthropic: `cache_creation_input_tokens`.
	/// Tokens used to build the cache (not yet cached). These may incur a small surcharge; subsequent requests benefit via `cached_tokens`.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cache_creation_tokens: Option<i32>,
	/// Anthropic: `cache_read_input_tokens`.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub cached_tokens: Option<i32>,
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub audio_tokens: Option<i32>,
}

impl PromptTokensDetails {
	/// True if all fields are `None`.
	pub fn is_empty(&self) -> bool {
		self.cache_creation_tokens.is_none() && self.cached_tokens.is_none() && self.audio_tokens.is_none()
	}
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
	/// Number of web search requests performed (Anthropic only).
	/// Billed at $10 per 1,000 searches.
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub web_search_requests: Option<i32>,

	/// Number of web fetch requests performed (Anthropic only).
	#[serde(default, deserialize_with = "crate::support::zero_as_none")]
	pub web_fetch_requests: Option<i32>,
}

impl CompletionTokensDetails {
	/// True if all fields are `None`.
	pub fn is_empty(&self) -> bool {
		self.accepted_prediction_tokens.is_none()
			&& self.rejected_prediction_tokens.is_none()
			&& self.reasoning_tokens.is_none()
			&& self.audio_tokens.is_none()
			&& self.web_search_requests.is_none()
			&& self.web_fetch_requests.is_none()
	}
}
