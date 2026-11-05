use super::*;
use serde_json::{Value, json};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn test_gemini_ix_usage_full_breakdown_ok() -> Result<()> {
	// -- Setup & Fixtures
	// Tool-use tokens sit outside `total_input_tokens`, so they are folded into the prompt.
	// Image tokens have no genai field and must not leak into the audio breakdown.
	let body = json!({
		"total_input_tokens": 100,
		"input_tokens_by_modality": [
			{"modality": "text", "tokens": 22},
			{"modality": "image", "tokens": 38},
			{"modality": "audio", "tokens": 40}
		],
		"total_cached_tokens": 30,
		"total_output_tokens": 20,
		"output_tokens_by_modality": [
			{"modality": "text", "tokens": 10},
			{"modality": "audio", "tokens": 10}
		],
		"total_thought_tokens": 50,
		"total_tool_use_tokens": 40,
		"tool_use_tokens_by_modality": [
			{"modality": "text", "tokens": 35},
			{"modality": "audio", "tokens": 5}
		],
		"total_tokens": 210
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	assert_eq!(usage.prompt_tokens, Some(140));
	assert_eq!(usage.completion_tokens, Some(70));
	assert_eq!(usage.total_tokens, Some(210));
	assert_eq!(
		usage.prompt_tokens_details,
		Some(PromptTokensDetails {
			cache_creation_tokens: None,
			cache_creation_details: None,
			cached_tokens: Some(30),
			audio_tokens: Some(45),
		})
	);
	assert_eq!(
		usage.completion_tokens_details,
		Some(CompletionTokensDetails {
			accepted_prediction_tokens: None,
			rejected_prediction_tokens: None,
			reasoning_tokens: Some(50),
			audio_tokens: Some(10),
		})
	);

	Ok(())
}

#[test]
fn test_gemini_ix_usage_zero_values_are_not_reported_ok() -> Result<()> {
	// -- Setup & Fixtures
	// Zero counters carry no information, so they must not create empty details objects.
	let body = json!({
		"total_input_tokens": 10,
		"input_tokens_by_modality": [{"modality": "audio", "tokens": 0}],
		"total_cached_tokens": 0,
		"total_output_tokens": 34,
		"total_thought_tokens": 0,
		"total_tool_use_tokens": 0,
		"total_tokens": 44
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	assert_eq!(usage.prompt_tokens, Some(10));
	assert_eq!(usage.completion_tokens, Some(34));
	assert_eq!(usage.total_tokens, Some(44));
	assert_eq!(usage.prompt_tokens_details, None);
	assert_eq!(usage.completion_tokens_details, None);

	Ok(())
}

#[test]
fn test_gemini_ix_usage_missing_total_is_computed_ok() -> Result<()> {
	// -- Setup & Fixtures
	let body = json!({
		"total_input_tokens": 7,
		"total_output_tokens": 23,
		"total_thought_tokens": 49
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	// Computed as prompt + completion, where completion already includes the thought tokens.
	assert_eq!(usage.total_tokens, Some(79));

	Ok(())
}

#[test]
fn test_gemini_ix_usage_reported_total_wins_ok() -> Result<()> {
	// -- Setup & Fixtures
	// The API total is kept even when it does not match the parts. The unknown `raw_prompt_token`
	// field must not break parsing.
	let body = json!({
		"total_input_tokens": 10,
		"total_output_tokens": 34,
		"total_tokens": 525,
		"raw_prompt_token": 41
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	assert_eq!(usage.total_tokens, Some(525));
	assert_eq!(usage.prompt_tokens, Some(10));

	Ok(())
}

#[test]
fn test_gemini_ix_usage_audio_input_only_ok() -> Result<()> {
	// -- Setup & Fixtures
	// A transcription-style call: audio in, text out, no cache and no thinking.
	let body = json!({
		"total_input_tokens": 1200,
		"input_tokens_by_modality": [{"modality": "audio", "tokens": 1200}],
		"total_output_tokens": 80,
		"output_tokens_by_modality": [{"modality": "text", "tokens": 80}],
		"total_thought_tokens": 0,
		"total_tokens": 1280
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	assert_eq!(
		usage.prompt_tokens_details,
		Some(PromptTokensDetails {
			cache_creation_tokens: None,
			cache_creation_details: None,
			cached_tokens: None,
			audio_tokens: Some(1200),
		})
	);
	assert_eq!(usage.completion_tokens_details, None);
	assert_eq!(usage.total_tokens, Some(1280));

	Ok(())
}

#[test]
fn test_gemini_ix_usage_thought_only_output_ok() -> Result<()> {
	// -- Setup & Fixtures
	// A turn halted while still thinking can report thought tokens without an output counter.
	// The thought tokens must still land in completion_tokens and in the computed total.
	let body = json!({
		"total_input_tokens": 12,
		"total_thought_tokens": 64
	});

	// -- Exec
	let usage = support_usage(body)?;

	// -- Check
	assert_eq!(usage.completion_tokens, Some(64));
	assert_eq!(
		usage.completion_tokens_details.as_ref().and_then(|d| d.reasoning_tokens),
		Some(64)
	);
	assert_eq!(usage.total_tokens, Some(76));

	Ok(())
}

// region:    --- Test Support

fn support_usage(body: Value) -> Result<Usage> {
	let ix_usage: IxUsage = serde_json::from_value(body)?;
	Ok(Usage::from(ix_usage))
}

// endregion: --- Test Support
