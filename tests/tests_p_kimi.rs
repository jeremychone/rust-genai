mod support;

use crate::support::{TestResult, common_tests};
use genai::resolver::AuthData;

const MODEL: &str = "kimi-k2.7-code";
const MODEL_NS: &str = "kimi::kimi-k2.6";
// const MODEL_IMAGE: &str = "minimax::MiniMax-M3";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

// NOTE: 2026-06-01 - Does not seem to be supported
// #[tokio::test]
// async fn test_chat_json_mode_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
// }

// NOTE: 2026-06-01 - Does not seem to be supported
// #[tokio::test]
// async fn test_chat_json_structured_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
// }

// NOTE: 2026-06-22 - only 1 is allowed for kimi 2p7-code and 2p6
// #[tokio::test]
// async fn test_chat_temperature_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_temperature_ok(MODEL).await
// }

// NOTE: 2026-06-01 - Does not seem to be supported
// #[tokio::test]
// async fn test_chat_stop_sequences_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
// }

// endregion: --- Chat

// region:    --- Chat Implicit Cache

/// Caching does not seem to be supported for fireworks (at leat not reported)
// #[tokio::test]
// async fn test_chat_cache_implicit_simple_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_cache_implicit_simple_ok(MODEL).await
// }

// endregion: --- Chat Implicit Cache

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

// NOTE: 2026-06-01 - Weird, described a totally different image
// #[tokio::test]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL_IMAGE).await
// }

// #[tokio::test]
// async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_b64_ok(MODEL_IMAGE).await
// }

// PDF not supported for fireworks.ai

// #[tokio::test]
// async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
// }

// #[tokio::test]
// async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
// }

#[tokio::test]
async fn test_chat_binary_video_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_video_b64_ok(MODEL).await
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
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("KIMI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

// NOTE: 2026-06-01 - Does not seem to be supported
#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	common_tests::common_test_list_models(genai::adapter::AdapterKind::Kimi, "kimi-k2.7-code").await
}

// endregion: --- List
