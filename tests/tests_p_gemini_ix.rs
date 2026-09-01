mod support;

use crate::support::{Check, TestResult, common_tests};
use genai::adapter::AdapterKind;
use genai::resolver::AuthData;

// The Interactions API is opt-in, so every model name here carries the `gemini_ix::` prefix.
const MODEL: &str = "gemini_ix::gemini-3.5-flash";
const MODEL_NS: &str = "gemini_ix::gemini-3.5-flash";

// region:    --- Provider Specific

#[tokio::test]
async fn test_gemini_ix_routing_ok() -> TestResult<()> {
	// -- Exec & Check
	assert_eq!(AdapterKind::from_model(MODEL)?, AdapterKind::GeminiIx);
	assert_eq!(AdapterKind::from_model("gemini-2.5-flash")?, AdapterKind::Gemini);
	// The escape hatch for anyone who wants `generateContent` for a Gemini 3 model.
	assert_eq!(
		AdapterKind::from_model("gemini::gemini-3.5-flash")?,
		AdapterKind::Gemini
	);

	Ok(())
}

#[tokio::test]
async fn test_gemini_ix_stateful_session_ok() -> TestResult<()> {
	use genai::chat::ChatRequest;

	// -- Setup & Fixtures
	let client = genai::Client::new()?;

	// -- Exec: turn 1. `store` defaults to true, but set it explicitly here so the test states
	//    its own requirement rather than leaning on the default.
	let chat_req = ChatRequest::from_user("My name is Vagmi. Reply with just 'ok'.").with_store(true);
	let res_1 = client.exec_chat(MODEL, chat_req, None).await?;
	let interaction_id = res_1.response_id.clone().ok_or("Should have a response_id")?;

	// -- Exec: turn 2 — no history resent, only the new turn.
	let chat_req = ChatRequest::from_user("What is my name?")
		.with_previous_response_id(&interaction_id)
		.with_store(true);
	let res_2 = client.exec_chat(MODEL, chat_req, None).await?;

	// -- Check
	let res_txt = res_2.into_first_text().ok_or("Should have result")?;
	assert!(
		res_txt.contains("Vagmi"),
		"the server should have recalled the name from the previous interaction. Got: {res_txt}"
	);

	Ok(())
}

#[tokio::test]
async fn test_gemini_ix_stateful_stream_captures_response_id_ok() -> TestResult<()> {
	use futures::StreamExt;
	use genai::chat::{ChatRequest, ChatStreamEvent};

	// -- Setup & Fixtures
	let client = genai::Client::new()?;
	let chat_req = ChatRequest::from_user("Say 'hello'.").with_store(true);

	// -- Exec
	let mut chat_res = client.exec_chat_stream(MODEL, chat_req, None).await?;
	let mut response_id: Option<String> = None;
	while let Some(event) = chat_res.stream.next().await {
		if let ChatStreamEvent::End(end) = event? {
			response_id = end.captured_response_id;
		}
	}

	// -- Check
	assert!(response_id.is_some(), "the stream end should carry the interaction id");

	Ok(())
}

// endregion: --- Provider Specific

// region:    --- Chat

#[tokio::test]
async fn test_chat_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_namespaced_ok() -> TestResult<()> {
	common_tests::common_test_chat_simple_ok(MODEL_NS, None).await
}

#[tokio::test]
async fn test_chat_top_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_top_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_multi_system_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_system_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_json_mode_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_mode_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_json_structured_ok() -> TestResult<()> {
	common_tests::common_test_chat_json_structured_ok(MODEL, Some(Check::USAGE)).await
}

#[tokio::test]
async fn test_chat_temperature_ok() -> TestResult<()> {
	common_tests::common_test_chat_temperature_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stop_sequences_ok() -> TestResult<()> {
	common_tests::common_test_chat_stop_sequences_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_reasoning_normalize_ok() -> TestResult<()> {
	common_tests::common_test_chat_reasoning_normalize_ok(MODEL).await
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

#[tokio::test]
async fn test_chat_stream_simple_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_simple_ok(MODEL, None).await
}

#[tokio::test]
async fn test_chat_stream_capture_content_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_content_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_stream_capture_all_ok() -> TestResult<()> {
	common_tests::common_test_chat_stream_capture_all_ok(MODEL, None).await
}

// endregion: --- Chat Stream Tests

// region:    --- Binary Tests

#[tokio::test]
async fn test_chat_binary_image_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_image_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_pdf_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_pdf_b64_ok(MODEL).await
}

#[tokio::test]
async fn test_chat_binary_multi_b64_ok() -> TestResult<()> {
	common_tests::common_test_chat_multi_binary_b64_ok(MODEL).await
}

// NOTE: Audio input is exercised by `examples/c99-gemini-transcribe.rs`, not here — it needs a
//       local audio fixture that cannot be committed (`*.wav` is gitignored), so as a test it
//       would silently skip on most checkouts.

// endregion: --- Binary Tests

// region:    --- Tool Tests

#[tokio::test]
async fn test_tool_simple_ok() -> TestResult<()> {
	common_tests::common_test_tool_simple_ok(MODEL).await
}

#[tokio::test]
async fn test_tool_full_flow_ok() -> TestResult<()> {
	common_tests::common_test_tool_full_flow_ok(MODEL).await
}

// endregion: --- Tool Tests

// region:    --- Resolver Tests

#[tokio::test]
async fn test_resolver_auth_ok() -> TestResult<()> {
	common_tests::common_test_resolver_auth_ok(MODEL, AuthData::from_env("GEMINI_API_KEY")).await
}

// endregion: --- Resolver Tests

// region:    --- List

#[tokio::test]
async fn test_list_models() -> TestResult<()> {
	// The listing is the Gemini `models` endpoint, filtered to what routes here.
	common_tests::common_test_list_models(AdapterKind::GeminiIx, "gemini-3.1-flash-lite").await
}

// endregion: --- List
