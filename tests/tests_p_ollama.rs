mod support;

use crate::support::common_tests;
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "llama3.2:3b"; // phi3:latest
const EMBEDDING_MODEL: &str = "nomic-embed-text";

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
async fn test_chat_json_mode_ok() -> Result<()> {
	// Note: Ollama does not capture Uage when JSON mode.
	common_tests::common_test_chat_json_mode_ok(MODEL, None).await
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
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// /// COMMENTED FOR NOW AS OLLAMA OpenAI Compatibility Layer does not support
// /// usage tokens when streaming. See https://github.com/ollama/ollama/issues/4448
// #[tokio::test]
// async fn test_chat_stream_capture_all_ok() -> Result<()> {
// 	common_tests::common_test_chat_stream_capture_all_ok(MODEL).await
// }

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_single("ollama")).await
}

// endregion: --- Resolver Tests

// region:    --- List

/// NOTE this test assume the "llama3.1:8b" is present.
#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Ollama, "llama3.1:8b").await
}

// endregion: --- List

// region:    --- Embed

#[tokio::test]
async fn test_embed_single() -> Result<()> {
	common_tests::test_embed_single(EMBEDDING_MODEL, None).await
}

#[tokio::test]
async fn test_embed_multiple() -> Result<()> {
	common_tests::test_embed_multiple(EMBEDDING_MODEL, None).await
}

// endregion: --- Embed
