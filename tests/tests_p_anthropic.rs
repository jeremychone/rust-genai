mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

// "claude-3-haiku-20240307" cheapest
// "claude-3-5-haiku-latest"
// "claude-3-7-sonnet-latest" (fail on test_chat_json_mode_ok)
// "claude-sonnet-4-20250514" (fail on test_chat_json_mode_ok)
//
const MODEL: &str = "claude-3-5-haiku-latest";
const MODEL_NS: &str = "anthropic::claude-3-5-haiku-latest";

// region:    --- Chat

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

/// TODO: Fix/Workaround - This test for "claude-3-7-sonnet-latest" (works for other models)
#[tokio::test]
#[serial(anthropic)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

// endregion: --- Chat

// region:    --- Chat Explicit Cache

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_cache_explicit_user_ok() -> TestResult<()> {
	common_tests::common_test_chat_cache_explicit_user_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_cache_explicit_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_cache_explicit_system_ok(MODEL).await
}

// endregion: --- Chat Explicit Cache

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}
// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

// NOTE: For now disable these tests as they failed. Needs to be resolved.

// Anthropic does not support image URL
// #[tokio::test]
// async fn test_chat_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
}

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
// #[serial(anthropic)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("ANTHROPIC_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(AdapterKind::Anthropic, "claude-3-7-sonnet-latest").await
}

// endregion: --- List
