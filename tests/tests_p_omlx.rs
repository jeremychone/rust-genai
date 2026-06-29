//! Integration tests for the Omlx adapter.
//!
//! NOTE: These tests require a live Omlx service and valid credentials.
//! Update the model name constants below to match available Omlx models.
//! Set the `OMLX_API_KEY` environment variable before running.

mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::chat::ReasoningEffort;
use genai::resolver::AuthData;

// -- Model constants
// export OMLX_API_KEY="welcome"
// export OMLX_ENDPOINT="http://127.0.0.1:8080/v1"
// NOTE: Update these to actual Omlx model names before running tests.
//       The adapter can use either namespaced form (`omlx::model`) or direct prefix (`omlx-model`).
const MODEL: &str = "omlx::Qwen3.6-35B-A3B-8bit";
const MODEL_REASONING: &str = "omlx::Qwen3.6-35B-A3B-8bit";
const MODEL_IMAGE: &str = "omlx::Qwen3.6-35B-A3B-8bit";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_reasoning_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_ok(
		MODEL_REASONING,
		ReasoningEffort::High,
		Some(Check::REASONING_CONTENT | Check::REASONING_USAGE),
	)
	.await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL_IMAGE).await
}

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("OMLX_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Omlx, "omlx::Qwen3.6-35B-A3B-8bit").await
}

// endregion: --- List
