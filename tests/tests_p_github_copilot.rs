mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::Client;
use genai::adapter::AdapterKind;
use serial_test::serial;

const MODEL: &str = "github_copilot::openai/gpt-5";
const MODEL_NS: &str = "github_copilot::meta/meta-llama-3.1-8b-instruct";

// NOTE: GitHub Copilot supports multiple publishers (openai/, anthropic/, google/, xai/, meta/).
// Tests use openai/gpt-5 as the default. Other publishers work with the same adapter.

// region:    --- Chat

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

// GitHub Copilot API does not emit streaming token counts for these models,
// so capture-all cannot satisfy the shared nonzero-usage assertion.
// See yakbak coverage in `tests/tests_yakbak_github_copilot.rs`.

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_stream_tool_capture_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_tool_capture_ok(MODEL).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

// GitHub Copilot API: base64 images supported; URL images fail (proxy issue); PDF not supported

// URL-based images fail — GitHub proxy returns "Failed to download image" (HTTP 400)
// #[tokio::test]
// async fn test_chat_binary_image_url_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_image_url_ok(MODEL).await
// }

#[tokio::test]
#[serial(github_copilot)]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

// PDF binary not supported by GitHub Copilot API
// #[tokio::test]
// async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
// }

// Multi-binary includes PDF — PDF not supported by GitHub Copilot API
// #[tokio::test]
// async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
// 	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
// }

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
#[serial(github_copilot)]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
#[serial(github_copilot)]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- List

#[tokio::test]
#[serial(github_copilot)]
async fn test_list_models() -> TestResult<()> {
	let client = Client::default();
	let models = client.all_model_names(AdapterKind::GithubCopilot).await?;
	assert!(
		models.is_empty(),
		"GitHub Copilot does not expose a model catalog; expected empty list"
	);
	Ok(())
}

// endregion: --- List
