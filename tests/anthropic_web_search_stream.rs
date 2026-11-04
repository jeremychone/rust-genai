//! A streamed Anthropic answer that uses the web search server tool carries
//! `server_tool_use` and `web_search_tool_result` blocks and `citations_delta`
//! events in its text blocks. None of these has a `/delta/text`, so they are
//! skipped and the text around them still streams to the end.

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

fn sse(events: &[serde_json::Value]) -> String {
	let mut body = String::new();
	for event in events {
		body.push_str("event: ");
		body.push_str(event["type"].as_str().expect("event type"));
		body.push_str("\ndata: ");
		body.push_str(&event.to_string());
		body.push_str("\n\n");
	}
	body
}

fn message_start() -> serde_json::Value {
	serde_json::json!({
		"type": "message_start",
		"message": {
			"id": "msg_1",
			"type": "message",
			"role": "assistant",
			"model": "claude-opus-5-5",
			"content": [],
			"stop_reason": null,
			"usage": { "input_tokens": 478, "output_tokens": 16 }
		}
	})
}

fn message_end(stop_reason: &str) -> Vec<serde_json::Value> {
	vec![
		serde_json::json!({
			"type": "message_delta",
			"delta": { "stop_reason": stop_reason, "stop_sequence": null },
			"usage": { "output_tokens": 120 }
		}),
		serde_json::json!({ "type": "message_stop" }),
	]
}

fn block_start(index: u32, content_block: serde_json::Value) -> serde_json::Value {
	serde_json::json!({ "type": "content_block_start", "index": index, "content_block": content_block })
}

fn delta(index: u32, delta: serde_json::Value) -> serde_json::Value {
	serde_json::json!({ "type": "content_block_delta", "index": index, "delta": delta })
}

fn block_stop(index: u32) -> serde_json::Value {
	serde_json::json!({ "type": "content_block_stop", "index": index })
}

/// The text chunks of a streamed answer, and whether it ended with an `End` event.
async fn text_of(events: &[serde_json::Value]) -> (String, bool) {
	let port = serve(sse(events));
	let client = Client::builder().build().expect("client");
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-5-5"),
	};
	let options = ChatOptions::default().with_capture_content(true);
	let response = client
		.exec_chat_stream(target, ChatRequest::from_user("hi"), Some(&options))
		.await
		.expect("stream");

	let mut stream = response.stream;
	let mut text = String::new();
	let mut ended = false;
	while let Some(event) = stream.next().await {
		match event.expect("a web search stream must not fail") {
			ChatStreamEvent::Chunk(chunk) => text.push_str(&chunk.content),
			ChatStreamEvent::End(_) => ended = true,
			_ => {}
		}
	}
	(text, ended)
}

#[tokio::test]
async fn web_search_blocks_and_citations_do_not_break_the_stream() {
	let mut events = vec![
		message_start(),
		block_start(0, serde_json::json!({ "type": "text", "text": "" })),
		delta(
			0,
			serde_json::json!({ "type": "text_delta", "text": "Let me search. " }),
		),
		block_stop(0),
		block_start(
			1,
			serde_json::json!({ "type": "server_tool_use", "id": "srvtoolu_1", "name": "web_search", "input": {} }),
		),
		delta(
			1,
			serde_json::json!({ "type": "input_json_delta", "partial_json": "{\"query\": \"rust\"}" }),
		),
		block_stop(1),
		block_start(
			2,
			serde_json::json!({ "type": "web_search_tool_result", "tool_use_id": "srvtoolu_1", "content": [] }),
		),
		block_stop(2),
		block_start(3, serde_json::json!({ "type": "text", "text": "" })),
		delta(
			3,
			serde_json::json!({ "type": "text_delta", "text": "Rust is a language." }),
		),
		delta(
			3,
			serde_json::json!({
				"type": "citations_delta",
				"citation": { "type": "web_search_result_location", "url": "https://rust-lang.org", "title": "Rust", "cited_text": "Rust", "encrypted_index": "x" }
			}),
		),
		block_stop(3),
	];
	events.extend(message_end("end_turn"));

	let (text, ended) = text_of(&events).await;

	assert_eq!(text, "Let me search. Rust is a language.");
	assert!(ended);
}
