mod support;

use crate::support::{Check, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "glm-z1-flash";

// region:    --- Chat

// NOTE - Disabled for now, as the model does not add .reasoning_content. Instead, it uses a <think> tag, which is tested in test_chat_reasoning_normalize_ok as of 2025-07-08.
// #[tokio::test]
// async fn test_chat_simple_ok() -> Result<()> {
// 	common_tests::common_test_chat_simple_ok(MODEL, Some(Check::REASONING)).await
// }

#[tokio::test]
async fn test_chat_multi_system_ok() -> Result<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_mode_ok() -> Result<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

/// NOTE - Disabled for now, as the model currently includes the stop sequences as the last sequences in the generation as of 2025-07-08.
// #[tokio::test]
// async fn test_chat_stop_sequences_ok() -> Result<()> {
// 	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
// }

#[tokio::test]
async fn test_chat_reasoning_normalize_ok() -> Result<()> {
	common_tests::common_test_chat_reasoning_normalize_ok(MODEL).await
}
// endregion: --- Chat

// region:    --- Chat Stream Tests

// NOTE - Disabled for now, as the model does not add .reasoning_content. Instead, it uses a <think> tag, which is tested in test_chat_reasoning_normalize_ok as of 2025-07-08.
// #[tokio::test]
// async fn test_chat_stream_simple_ok() -> Result<()> {
// 	common_tests::common_test_chat_stream_simple_ok(MODEL, Some(Check::REASONING)).await
// }

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// NOTE - Disabled for now, as the model does not add .reasoning_content. Instead, it uses a <think> tag, which is tested in test_chat_reasoning_normalize_ok as of 2025-07-08.
// #[tokio::test]
// async fn test_chat_stream_capture_all_ok() -> Result<()> {
// 	common_tests::common_test_chat_stream_capture_all_ok(MODEL, Some(Check::REASONING)).await
// }

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("ZHIPU_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Zhipu, "glm-z1-flash").await
}

// endregion: --- List
