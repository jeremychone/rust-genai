mod support;

use crate::support::{Check, common_tests};
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// const MODEL: &str = "together::meta-llama/Llama-3-8b-chat-hf";
// meta-llama/Llama-3.3-70B-Instruct-Turbo
const MODEL: &str = "together::meta-llama/Llama-3.3-70B-Instruct-Turbo";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

// Only namspaced for together
// #[tokio::test]
// async fn test_chat_namespaced_ok() -> Result<()> {
// 	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
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

/// Caching does not seem to be supported for fireworks (at leat not reported)
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

#[tokio::test]
async fn test_chat_image_url_ok() -> Result<()> {
	common_tests::common_test_chat_image_url_ok(MODEL).await
}

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

// NOTE for now not working with Llama-3.3-70B-Instruct-Turbo
// TODO: need to investigate
// #[tokio::test]
// async fn test_tool_full_flow_ok() -> Result<()> {
// 	common_tests::common_test_tool_full_flow_ok(MODEL, true).await
// }
// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("TOGETHER_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

// #[tokio::test]
// async fn test_list_models() -> Result<()> {
// 	//common_tests::common_test_list_models(AdapterKind::Fireworks, "..").await
// }

// endregion: --- List
