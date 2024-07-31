mod support;

use crate::support::common_tests;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "phi3:latest";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_ok() -> Result<()> {
	common_tests::common_test_chat_json_ok(MODEL, false).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> Result<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// /// COMMENTED FOR NOW AS OLLAMA OpenAI Compatibility Layer does not support
// /// usage token when streaming. See https://github.com/ollama/ollama/issues/4448
// #[tokio::test]
// async fn test_chat_stream_capture_all_ok() -> Result<()> {
// 	common_test_chat_stream_capture_all_ok(MODEL).await
// }

// endregion: --- Chat Stream Tests
