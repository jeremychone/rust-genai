//! Live tests for `bedrock_api` (Bedrock Converse with a Bearer API key).
//!
//! Needs `BEDROCK_API_KEY` or `AWS_BEARER_TOKEN_BEDROCK`. `AWS_REGION` defaults to `us-east-1`.
//!
//! Run with: `cargo test --test tests_p_bedrock_api -- --nocapture`
//!
//! The reasoning + tool round trip is covered by the replay tests in `tests_yakbak_bedrock.rs`.

mod support;

use crate::support::{TestResult, common_tests};
use genai::chat::ReasoningEffort;
use serial_test::serial;

// Cross-region inference profile: newer models are only reachable through one.
const MODEL: &str = "bedrock_api::global.openai.gpt-6-luna";

const MODEL_OPENAI: &str = "bedrock_api::global.openai.gpt-6-luna";
const MODEL_ANTHROPIC: &str = "bedrock_api::us.anthropic.claude-haiku-5-5";

// Bare (non-profile) id, to exercise publisher detection without a geography prefix.
const MODEL_BARE: &str = "bedrock_api::amazon.nova-lite-v1:0";
// Nova 2 reasons with `reasoningConfig.maxReasoningEffort`.
const MODEL_NOVA_2: &str = "bedrock_api::us.amazon.nova-2-lite-v1:0";
// Claude with adaptive thinking: rejects `thinking.type: enabled`, needs `output_config.effort`.
const MODEL_CLAUDE_ADAPTIVE: &str = "bedrock_api::us.anthropic.claude-opus-5-5";

// region:    --- Chat

#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_bare_model_id_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_BARE, None).await
}

/// Nova 2 needs `maxReasoningEffort` whenever reasoning is enabled.
#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_reasoning_nova_medium_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_ok(MODEL_NOVA_2, ReasoningEffort::Medium, None).await
}

/// High effort: Nova rejects maxTokens, temperature and topP alongside it.
#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_reasoning_nova_high_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_ok(MODEL_NOVA_2, ReasoningEffort::High, None).await
}

/// Adaptive-thinking Claude takes `thinking.type: adaptive` + `output_config.effort`.
#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_reasoning_claude_adaptive_high_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_ok(MODEL_CLAUDE_ADAPTIVE, ReasoningEffort::High, None).await
}

/// Opus 5 rejects `thinking.type: disabled`, so `Zero` must send no reasoning fields.
#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_reasoning_claude_adaptive_zero_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_ok(MODEL_CLAUDE_ADAPTIVE, ReasoningEffort::Zero, None).await
}

// endregion: --- Chat

// region:    --- Chat Stream

#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

#[tokio::test]
#[serial(bedrock_api)]
async fn test_chat_stream_tool_capture_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_tool_capture_ok(MODEL).await
}

// endregion: --- Chat Stream

// region:    --- Binary Tests

/// NOTE: OpenAI - Does not see to get the image, describe something random (might be AWS settings)
/// NOTE: Anthropic - say `I don't see an image attached to your message. Could you try uploading it again?` but it's a link??
// #[tokio::test]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL_ANTHROPIC).await
// }

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	// openai
	common_tests::common_test_chat_image_b64_ok(MODEL_OPENAI).await?;
	// anthropic
	common_tests::common_test_chat_image_b64_ok(MODEL_ANTHROPIC).await?;
	Ok(())
}

#[tokio::test]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	// openai (PDF NOT AVAILABLE on AWS)
	// common_tests::common_test_chat_pdf_b64_ok(MODEL_OPENAI).await?;

	// anthropic
	common_tests::common_test_chat_pdf_b64_ok(MODEL_ANTHROPIC).await?;

	Ok(())
}

#[tokio::test]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	// openai (PDF not supported, and this request as image & pdf doc)
	// common_tests::common_test_chat_multi_binary_b64_ok(MODEL_OPENAI).await?;

	// anthropic
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL_ANTHROPIC).await?;

	Ok(())
}

// No video model setup yet.
// #[tokio::test]
// async fn test_chat_binary_audio_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_audio_b64_ok(MODEL).await
// }

// No video model setup yet.
// #[tokio::test]
// async fn test_chat_binary_video_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_video_b64_ok(MODEL).await
// }

// endregion: --- Binary Tests

// region:    --- Tools

#[tokio::test]
#[serial(bedrock_api)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tools
