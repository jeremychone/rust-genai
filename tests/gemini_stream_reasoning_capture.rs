//! A streamed Gemini answer reports its thought parts as reasoning chunks, and
//! `capture_reasoning_content` alone keeps them in the captured end, joined in
//! order when one chunk carries several thought parts.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};

use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatOptions, ChatRequest, ChatStreamEvent};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ServiceTarget};

fn serve(body: String) -> u16 {
	let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
	let port = listener.local_addr().expect("addr").port();
	std::thread::spawn(move || {
		let (mut stream, _) = listener.accept().expect("accept");
		let mut buf = [0_u8; 8192];
		let _ = stream.read(&mut buf);
		let header = format!(
			"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
			body.len()
		);
		stream.write_all(header.as_bytes()).expect("header");
		stream.write_all(body.as_bytes()).expect("body");
		let _ = stream.shutdown(Shutdown::Both);
	});
	port
}

/// One Gemini SSE frame with the given parts.
fn frame(parts: serde_json::Value, finish: bool) -> serde_json::Value {
	let mut candidate = serde_json::json!({ "content": { "role": "model", "parts": parts }, "index": 0 });
	if finish {
		candidate["finishReason"] = serde_json::json!("STOP");
	}
	serde_json::json!({
		"candidates": [candidate],
		"usageMetadata": { "promptTokenCount": 3, "candidatesTokenCount": 5, "totalTokenCount": 8 }
	})
}

fn sse_frames(frames: &[serde_json::Value]) -> String {
	frames.iter().map(|f| format!("data: {f}\n\n")).collect()
}

/// The reasoning chunks streamed and the reasoning and content captured at the end.
async fn run(options: ChatOptions) -> (String, Option<String>, Option<String>) {
	let frames = [
		frame(
			serde_json::json!([
				{ "text": "First thought. ", "thought": true },
				{ "text": "Second thought.", "thought": true }
			]),
			false,
		),
		frame(serde_json::json!([{ "text": "The answer." }]), true),
	];
	let port = serve(sse_frames(&frames));
	let client = Client::builder().build().expect("client");
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1beta/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::Gemini, "gemini-2.5-flash"),
	};
	let response = client
		.exec_chat_stream(target, ChatRequest::from_user("hi"), Some(&options))
		.await
		.expect("stream");

	let mut stream = response.stream;
	let mut streamed = String::new();
	let mut captured = (None, None);
	while let Some(event) = stream.next().await {
		match event.expect("event") {
			ChatStreamEvent::ReasoningChunk(chunk) => streamed.push_str(&chunk.content),
			ChatStreamEvent::End(end) => {
				captured = (
					end.captured_reasoning_content.clone(),
					end.captured_first_text().map(str::to_string),
				);
			}
			_ => {}
		}
	}
	(streamed, captured.0, captured.1)
}

#[tokio::test]
async fn capture_reasoning_content_alone_keeps_every_thought_part() {
	let (streamed, reasoning, content) = run(ChatOptions::default().with_capture_reasoning_content(true)).await;

	assert_eq!(streamed, "First thought. Second thought.");
	assert_eq!(reasoning.as_deref(), Some("First thought. Second thought."));
	assert_eq!(content, None);
}

#[tokio::test]
async fn capture_content_alone_does_not_capture_reasoning() {
	let (_, reasoning, content) = run(ChatOptions::default().with_capture_content(true)).await;

	assert_eq!(reasoning, None);
	assert_eq!(content.as_deref(), Some("The answer."));
}
