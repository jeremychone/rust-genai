//! A `chat.completion.chunk` can carry more than one piece of the answer: several
//! `delta.tool_calls` entries, or reasoning and content together. The OpenAI
//! streamer must emit and capture every piece, not just the first.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};

use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatOptions, ChatRequest, ChatStreamEvent, StreamEnd, ToolCall};
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

/// One `chat.completion.chunk` frame per delta, then `[DONE]`.
fn sse(deltas: &[(serde_json::Value, Option<&str>)]) -> String {
	let mut body = String::new();
	for (delta, finish_reason) in deltas {
		let chunk = serde_json::json!({
			"id": "chatcmpl-1",
			"object": "chat.completion.chunk",
			"model": "gpt-test",
			"choices": [{ "index": 0, "delta": delta, "finish_reason": finish_reason }]
		});
		body.push_str(&format!("data: {chunk}\n\n"));
	}
	body.push_str("data: [DONE]\n\n");
	body
}

/// Streams the frames and returns the events in order (without Start) and the end.
async fn run(deltas: &[(serde_json::Value, Option<&str>)]) -> (Vec<ChatStreamEvent>, StreamEnd) {
	let port = serve(sse(deltas));
	let client = Client::builder().build().expect("client");
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-test"),
	};
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true);
	let response = client
		.exec_chat_stream(target, ChatRequest::from_user("hi"), Some(&options))
		.await
		.expect("stream");

	let mut stream = response.stream;
	let mut events = Vec::new();
	let mut end = None;
	while let Some(event) = stream.next().await {
		match event.expect("event") {
			ChatStreamEvent::Start => {}
			ChatStreamEvent::End(stream_end) => end = Some(stream_end),
			other => events.push(other),
		}
	}
	(events, end.expect("stream end"))
}

fn tool_call_delta(index: u32, id: &str, city: &str) -> serde_json::Value {
	serde_json::json!({
		"index": index,
		"id": id,
		"type": "function",
		"function": { "name": "get_weather", "arguments": format!(r#"{{"city":"{city}"}}"#) }
	})
}

fn captured_calls(end: &StreamEnd) -> Vec<ToolCall> {
	end.captured_tool_calls()
		.map(|calls| calls.into_iter().cloned().collect())
		.unwrap_or_default()
}

#[tokio::test]
async fn two_tool_calls_in_one_chunk_are_both_kept() {
	let (events, end) = run(&[
		(serde_json::json!({ "role": "assistant" }), None),
		(
			serde_json::json!({ "tool_calls": [tool_call_delta(0, "call_a", "Paris"), tool_call_delta(1, "call_b", "Tokyo")] }),
			None,
		),
		(serde_json::json!({}), Some("tool_calls")),
	])
	.await;

	let event_ids: Vec<&str> = events
		.iter()
		.filter_map(|event| match event {
			ChatStreamEvent::ToolCallChunk(chunk) => Some(chunk.tool_call.call_id.as_str()),
			_ => None,
		})
		.collect();
	assert_eq!(event_ids, ["call_a", "call_b"]);

	let calls = captured_calls(&end);
	assert_eq!(calls.len(), 2);
	assert_eq!(calls[0].call_id, "call_a");
	assert_eq!(calls[0].fn_arguments, serde_json::json!({ "city": "Paris" }));
	assert_eq!(calls[1].call_id, "call_b");
	assert_eq!(calls[1].fn_arguments, serde_json::json!({ "city": "Tokyo" }));
}

#[tokio::test]
async fn tool_calls_with_finish_reason_each_get_an_event() {
	let (events, end) = run(&[(
		serde_json::json!({ "tool_calls": [tool_call_delta(0, "call_a", "Paris"), tool_call_delta(1, "call_b", "Tokyo")] }),
		Some("tool_calls"),
	)])
	.await;

	let tool_call_events = events
		.iter()
		.filter(|event| matches!(event, ChatStreamEvent::ToolCallChunk(_)))
		.count();
	assert_eq!(tool_call_events, 2);
	assert_eq!(captured_calls(&end).len(), 2);
}

#[tokio::test]
async fn reasoning_and_content_in_one_chunk_are_both_kept() {
	let (events, end) = run(&[
		(serde_json::json!({ "reasoning_content": "Think. " }), None),
		(
			serde_json::json!({ "reasoning_content": "Done.", "content": "Hel" }),
			None,
		),
		(serde_json::json!({ "content": "lo" }), None),
		(
			serde_json::json!({ "reasoning_content": "Late.", "content": "!" }),
			Some("stop"),
		),
	])
	.await;

	let pieces: Vec<String> = events
		.iter()
		.filter_map(|event| match event {
			ChatStreamEvent::ReasoningChunk(chunk) => Some(format!("r:{}", chunk.content)),
			ChatStreamEvent::Chunk(chunk) => Some(format!("c:{}", chunk.content)),
			_ => None,
		})
		.collect();
	assert_eq!(pieces, ["r:Think. ", "r:Done.", "c:Hel", "c:lo", "r:Late.", "c:!"]);

	assert_eq!(end.captured_reasoning_content.as_deref(), Some("Think. Done.Late."));
	assert_eq!(end.captured_first_text(), Some("Hello!"));
}

#[tokio::test]
async fn content_next_to_tool_calls_is_kept() {
	let (_events, end) = run(&[
		(
			serde_json::json!({ "content": "Checking.", "tool_calls": [tool_call_delta(0, "call_a", "Paris")] }),
			None,
		),
		(serde_json::json!({}), Some("tool_calls")),
	])
	.await;

	assert_eq!(end.captured_first_text(), Some("Checking."));
	assert_eq!(captured_calls(&end).len(), 1);
}
