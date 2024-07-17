mod support;

use crate::support::extract_stream_end;
use genai::chat::{ChatMessage, ChatRequest, ChatRequestOptions};
use genai::{Client, ClientConfig};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

const MODEL: &str = "gpt-3.5-turbo";

#[tokio::test]
async fn test_chat_stream_simple_ok() -> Result<()> {
	// -- Setup & Fixtures
	let client = Client::default();

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is sky blue?"),
	]);

	// -- Exec
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

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

#[tokio::test]
async fn test_chat_stream_with_captures_ok() -> Result<()> {
	// -- Setup & Fixtures
	let config = ClientConfig::default().with_default_chat_request_options(
		ChatRequestOptions::default()
			.with_capture_usage(true)
			.with_capture_content(true),
	);
	let client = Client::builder().with_config(config).build();

	let chat_req = ChatRequest::new(vec![
		// -- Messages (de/activate to see the differences)
		ChatMessage::system("Answer in one sentence"),
		ChatMessage::user("Why is sky blue?"),
	]);

	// -- Exec
	let chat_res = client.exec_chat_stream(MODEL, chat_req.clone(), None).await?;

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
