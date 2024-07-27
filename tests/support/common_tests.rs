use crate::get_option_value;
use crate::support::{extract_stream_end, seed_chat_req_simple, Result};
use genai::chat::ChatOptions;
use genai::Client;

// region:    --- Chat

pub async fn common_test_chat_simple_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat(Some(model), chat_req, None).await?;

	// -- Check
	assert!(
		!get_option_value!(chat_res.content).is_empty(),
		"content should not be empty"
	);
	let usage = chat_res.usage;
	let input_tokens = get_option_value!(usage.input_tokens);
	let output_tokens = get_option_value!(usage.output_tokens);
	let total_tokens = get_option_value!(usage.total_tokens);

	assert!(total_tokens > 0, "total_tokens should be > 0");
	assert!(
		total_tokens == input_tokens + output_tokens,
		"total_tokens should be input_tokens + output_tokens"
	);

	Ok(())
}

// endregion: --- Chat

// region:    --- Chat Stream Tests

pub async fn common_test_chat_stream_simple_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(Some(model), chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check no meta_usage and captured_content
	assert!(stream_end.captured_usage.is_none(), "StreamEnd not have any meta_usage");
	assert!(
		stream_end.captured_content.is_none(),
		"StreamEnd not have any captured_content"
	);

	Ok(())
}

pub async fn common_test_chat_stream_capture_content_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(Some(model), chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	// should be None as not captured
	assert!(stream_end.captured_usage.is_none(), "StreamEnd not have any meta_usage");

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.len should be > 0");

	Ok(())
}

pub async fn common_test_chat_stream_capture_all_ok(model: &str) -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::builder()
		.with_chat_options(ChatOptions::default().with_capture_usage(true).with_capture_content(true))
		.build();
	let chat_req = seed_chat_req_simple();

	// -- Exec
	let chat_res = client.exec_chat_stream(Some(model), chat_req.clone(), None).await?;

	// -- Check StreamEnd
	let stream_end = extract_stream_end(chat_res.stream).await?;

	// -- Check meta_usage
	let meta_usage = get_option_value!(stream_end.captured_usage);

	assert!(
		get_option_value!(meta_usage.input_tokens) > 0,
		"input_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.output_tokens) > 0,
		"output_tokens should be > 0"
	);
	assert!(
		get_option_value!(meta_usage.total_tokens) > 0,
		"total_tokens should be > 0"
	);

	// -- Check captured_content
	let captured_content = get_option_value!(stream_end.captured_content);
	assert!(!captured_content.is_empty(), "captured_content.len should be > 0");

	Ok(())
}

// endregion: --- Chat Stream Tests
