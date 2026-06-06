mod support;

use crate::support::{TestResult, assert_contains, common_tests};
use genai::{Client, adapter::AdapterKind};
use serial_test::serial;

const MODEL: &str = "genai_1::gemma-4-26b-a4b-it-4bit";

/// export GENAI_1_ENDPOINT="http://127.0.0.1:8000/v1"
/// export GENAI_1_API_KEY="welcome"

// region:    --- Chat

#[tokio::test]
#[serial(together)]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	// common_tests::common_test_list_models(AdapterKind::DeepSeek, "deepseek-v4-flash").await

	let client = Client::default();

	// -- Exec
	let models = client.all_model_names(AdapterKind::Custom(1), None).await?;

	// -- Check
	assert_contains(&models, "gemma-4-26b-a4b-it-4bit");

	Ok(())
}
