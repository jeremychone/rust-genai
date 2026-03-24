//! Recording scripts for yakbak cassettes.
//!
//! These are `#[ignore]` tests — run manually with real API keys:
//!
//! ```sh
//! OPENAI_API_KEY=... GEMINI_API_KEY=... cargo test --test tests_yakbak_record -- --ignored
//! ```
//!
//! Each test records a response cassette to `tests/data/yakbak/{provider}/{scenario}/`.

mod support;

use genai::chat::*;
use support::yakbak::record_client;
use support::{TestResult, extract_stream_end};
use serde_json::json;


fn openai_backend() -> String {
	std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1/".to_string())
}

fn gemini_backend() -> String {
	std::env::var("GEMINI_BASE_URL").unwrap_or_else(|_| "https://generativelanguage.googleapis.com/v1beta/".to_string())
}



const OPENAI_MODEL: &str = "openai_resp::gpt-5.4-mini";

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream", &openai_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream content: {:?}", extract.content.as_deref().map(|s| &s[..s.len().min(80)]));
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)]));

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_openai_resp_reasoning_stream_tools() -> TestResult<()> {
	let (client, mut server) = record_client("openai_resp", "reasoning_stream_tools", &openai_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(OPENAI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.is_some());
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}



const GEMINI_MODEL: &str = "gemini-2.5-flash";

#[tokio::test]
#[ignore]
async fn record_gemini_thinking_nostream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "thinking_nostream", &gemini_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default().with_reasoning_effort(ReasoningEffort::Low);

	let res = client.exec_chat(GEMINI_MODEL, chat_req, Some(&options)).await?;
	let text = res.first_text().ok_or("Should have text content")?;
	eprintln!("[record] Got text: {}", &text[..text.len().min(80)]);
	if let Some(rc) = &res.reasoning_content {
		eprintln!("[record] Got reasoning_content ({} chars)", rc.len());
	}

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_gemini_thinking_stream() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "thinking_stream", &gemini_backend()).await?;

	let chat_req = ChatRequest::new(vec![
		ChatMessage::system("Answer in one sentence."),
		ChatMessage::user("Why is the sky blue?"),
	]);
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(GEMINI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream content: {:?}", extract.content.as_deref().map(|s| &s[..s.len().min(80)]));
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.as_deref().map(|s| &s[..s.len().min(80)]));

	server.shutdown().await;
	Ok(())
}

#[tokio::test]
#[ignore]
async fn record_gemini_thinking_stream_tools() -> TestResult<()> {
	let (client, mut server) = record_client("gemini", "thinking_stream_tools", &gemini_backend()).await?;

	let chat_req = seed_tool_request();
	let options = ChatOptions::default()
		.with_reasoning_effort(ReasoningEffort::Low)
		.with_capture_content(true)
		.with_capture_reasoning_content(true);

	let stream_res = client.exec_chat_stream(GEMINI_MODEL, chat_req, Some(&options)).await?;
	let extract = extract_stream_end(stream_res.stream).await?;
	eprintln!("[record] Stream reasoning: {:?}", extract.reasoning_content.is_some());
	let tool_calls = &extract.stream_end.captured_tool_calls();
	eprintln!("[record] Tool calls: {:?}", tool_calls.as_ref().map(|tc| tc.len()));

	server.shutdown().await;
	Ok(())
}



fn seed_tool_request() -> ChatRequest {
	ChatRequest::new(vec![
		ChatMessage::system("You are a helpful assistant. Use tools when needed."),
		ChatMessage::user("What is the temperature in C and weather, in Paris, France"),
	])
	.append_tool(Tool::new("get_weather").with_schema(json!({
		"type": "object",
		"properties": {
			"city": {
				"type": "string",
				"description": "The city name"
			},
			"country": {
				"type": "string",
				"description": "The most likely country of this city name"
			},
			"unit": {
				"type": "string",
				"enum": ["C", "F"],
				"description": "The temperature unit of the country. C for Celsius, and F for Fahrenheit"
			}
		},
		"required": ["city", "country", "unit"],
	})))
}

