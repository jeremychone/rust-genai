mod support;

use crate::support::common_tests;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// Note: On groq, the llama3.1 or gemma models fail to produce JSON without a proposed schema.
//       With the "tool-use" groq version, it will work correctly.
const MODEL: &str = "llama3-groq-8b-8192-tool-use-preview";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_ok() -> Result<()> {
	common_tests::common_test_chat_json_ok(MODEL, true).await
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

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL).await
}

// endregion: --- Chat Stream Tests
