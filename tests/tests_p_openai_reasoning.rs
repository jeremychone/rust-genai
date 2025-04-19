mod support;

use crate::support::{Check, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

// NOTE 2025-01-31: OpenAI reasoning models do not provide reasoning right now, so for now we disabled those test(s)

// NOTE: We are still splitting out the openai_reasoning test to make sure we spot any disparity.

// "o4-mini" (or genai aliases "o4-mini-low", "o4-mini-medium", "o4-mini-high")
// Note "o4-mini-low" will be interpreted with `o4-mini` with `ChatOptions::default().with_reasoning_effort(ReasoningEffort::Low)`
const MODEL: &str = "o4-mini-low";

// When -low, might have no reasoning tokens when simple prompt, so using higher reasoning effort.
const MODEL_FOR_THINKING: &str = "o4-mini-medium";

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> Result<()> {
	// NOTE 2025-01-31  - Reasoning_content or <think> content not supported by OpenAI at this point
	//                    So, disabled for now
	common_tests::common_test_chat_simple_ok(MODEL_FOR_THINKING, Some(Check::REASONING_USAGE)).await
}

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

// NOTE 2025-01-31 - OpenAI reasoning model does not temperature
// #[tokio::test]
// async fn test_chat_temperature_ok() -> Result<()> {
// 	common_tests::common_test_chat_temperature_ok(MODEL).await
// }

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> Result<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

/// NOTE 2025-01-31  - Reasoning_content or <think> content not supported by OpenAI at this point
///                    So, disabled for now.
// #[tokio::test]
// async fn test_chat_reasoning_ok() -> Result<()> {
// 	common_tests::common_test_chat_reasoning_ok(MODEL, true).await
// }

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

// endregion: --- Chat Stream Tests

// region:    --- Image Tests

// NOTE 2025-01-31 - OpenAI reasoning model does not support image

// #[tokio::test]
// async fn test_chat_image_url_ok() -> Result<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

// #[tokio::test]
// async fn test_chat_image_b64_ok() -> Result<()> {
// 	common_tests::common_test_chat_image_b64_ok(MODEL).await
// }

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

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> Result<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("OPENAI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> Result<()> {
	common_tests::common_test_list_models(AdapterKind::OpenAI, "gpt-4o").await
}

// endregion: --- List
