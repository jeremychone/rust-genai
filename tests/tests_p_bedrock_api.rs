//! Live tests for `bedrock_api` (Bedrock Converse with a Bearer API key).
//!
//! Needs `BEDROCK_API_KEY` or `AWS_BEARER_TOKEN_BEDROCK`. `AWS_REGION` defaults to `us-east-1`.
//!
//! Run with: `cargo test --test tests_p_bedrock_api -- --nocapture`
//!
//! The reasoning + tool round trip is covered by the replay tests in `tests_yakbak_bedrock.rs`.

mod support;

use crate::support::{TestResult, common_tests};
use serial_test::serial;

// Cross-region inference profile: newer models are only reachable through one.
const MODEL: &str = "bedrock_api::global.openai.gpt-5.6-terra";
// Bare (non-profile) id, to exercise publisher detection without a geography prefix.
const MODEL_BARE: &str = "bedrock_api::amazon.nova-lite-v1:0";

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

// region:    --- Tools

#[tokio::test]
#[serial(bedrock_api)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tools
