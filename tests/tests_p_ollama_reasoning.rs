mod support;

use crate::support::common_tests;
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;
use serial_test::serial;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// NOTE: Sometimes the 1.5b model does not provide the reasoning or has some issues.
//       Rerunning the test or switching to the 8b model would generally solve the issues.

// NOTE: Also, #[serial(ollama)] seems more reliable when using it.

const MODEL: &str = "deepseek-r1:1.5b"; // "deepseek-r1:8b" "deepseek-r1:1.5b"

// region:    --- Chat

#[tokio::test]
#[serial(ollama)]
async fn test_chat_simple_ok() -> Result<()> {
	// NOTE: For now, the Ollama Deepseek Distilled model does not add .reasoning_content,
	//       but has a <think> tag which is tested in test_chat_reasoning_normalize_ok.
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_multi_system_ok() -> Result<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_json_mode_ok() -> Result<()> {
	// Note: Ollama does not capture usage on json mode (TODO: need to check now (2025-02-02))
	common_tests::common_test_chat_json_mode_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_temperature_ok() -> Result<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stop_sequences_ok() -> Result<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

/// Note: Unfortunately, sometime, the 1.5b does not provide reasoning.
#[tokio::test]
#[serial(ollama)]
async fn test_chat_reasoning_normalize_ok() -> Result<()> {
	common_tests::common_test_chat_reasoning_normalize_ok(MODEL).await
}
// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(ollama)]
async fn test_chat_stream_simple_ok() -> Result<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(ollama)]
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
#[serial(ollama)]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_single("ollama")).await
}

// endregion: --- Resolver Tests

// region:    --- List

/// NOTE this test assume the "gemma3:4b" is present.
#[tokio::test]
#[serial(ollama)]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::Ollama, "gemma3:4b").await
}

// endregion: --- List
