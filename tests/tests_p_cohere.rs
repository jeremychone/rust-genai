mod support;

use crate::support::common_tests;
use genai::adapter::AdapterKind;
use genai::embed::EmbedOptions;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "command-light";
const EMBEDDING_MODEL: &str = "embed-english-light-v3.0";

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

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> Result<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

// endregion: --- Chat Stream Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("COHERE_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Cohere, "command-r-plus").await
}

// endregion: --- List

// region:    --- Embed

#[tokio::test]
async fn test_embed_single() -> Result<()> {
	common_tests::test_embed_single(
		EMBEDDING_MODEL,
		Some(&EmbedOptions::default().with_input_type("search_query".to_string())),
	)
	.await
}

#[tokio::test]
async fn test_embed_multiple() -> Result<()> {
	common_tests::test_embed_multiple(
		EMBEDDING_MODEL,
		Some(&EmbedOptions::default().with_input_type("search_document".to_string())),
	)
	.await
}

// endregion: --- Embed
