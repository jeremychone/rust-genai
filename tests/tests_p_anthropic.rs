mod support;

use crate::support::{common_tests, Check};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// 4k (cheaper)
const MODEL: &str = "claude-3-5-haiku-latest";
// 8k output context
// const MODEL: &str = "claude-3-5-haiku-20241022";

// region:    --- Chat

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> Result<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stop_sequences_ok() -> Result<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_json_mode_ok() -> Result<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_simple_ok() -> Result<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_capture_content_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
#[serial(anthropic)]
async fn test_chat_stream_capture_all_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}
// endregion: --- Chat Stream Tests

// region:    --- Image Tests

// NOTE: For now disable these tests as they failed. Needs to be resolved.

// Anthropic does not support image URL
// #[tokio::test]
// async fn test_chat_image_url_ok() -> Result<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

#[tokio::test]
async fn test_chat_image_b64_ok() -> Result<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

// endregion: --- Image Test

// region:    --- Tool Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_tool_simple_ok() -> Result<()> {
	common_tests::common_test_tool_simple_ok(MODEL, false).await
}

#[tokio::test]
// #[serial(anthropic)]
async fn test_tool_full_flow_ok() -> Result<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL, false).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
#[serial(anthropic)]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("ANTHROPIC_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Anthropic, "claude-3-7-sonnet-latest").await
}

// endregion: --- List
