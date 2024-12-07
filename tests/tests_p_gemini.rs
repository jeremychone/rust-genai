mod support;

use crate::support::common_tests;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "gemini-1.5-flash-latest";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> Result<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, true).await
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

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("GEMINI_API_KEY")).await
}

// endregion: --- Resolver Tests
