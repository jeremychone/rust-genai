mod support;

use crate::support::{Check, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// "gemini-2.0-flash", "gemini-2.0-flash-lite" (somehow function calling work with -lite)
// "gemini-2.5-flash-preview-04-17" "gemini-2.5-pro-preview-05-06"
const MODEL: &str = "gemini-2.5-flash-preview-04-17-zero";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> Result<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> Result<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> Result<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Implicit Cache

// NOTE: This should eventually work with the 2.5 Flash, but right now, we do not get the cached_tokens
//       So, disabling
// #[tokio::test]
// async fn test_chat_cache_implicit_simple_ok() -> Result<()> {
// 	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL).await
// }

// endregion: --- Chat Implicit Cache

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> Result<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Image Tests

// NOTE: Gemini does not seem to support URL
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
async fn test_tool_simple_ok() -> Result<()> {
	common_tests::common_test_tool_simple_ok(MODEL, true).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> Result<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL, true).await
}
// endregion: --- Tool Tests

// region:    --- Typed Tool Tests

#[tokio::test]
async fn test_typed_tool_simple_ok() -> Result<()> {
	common_tests::common_test_typed_tool_simple_ok(MODEL, true).await
}

#[tokio::test]
async fn test_typed_tool_full_flow_ok() -> Result<()> {
	common_tests::common_test_typed_tool_full_flow_ok(MODEL, true).await
}

#[tokio::test]
async fn test_typed_tool_compatibility_ok() -> Result<()> {
	common_tests::common_test_typed_tool_compatibility_ok(MODEL).await
}

// endregion: --- Typed Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("GEMINI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Gemini, "gemini-1.5-pro").await
}

// endregion: --- List
