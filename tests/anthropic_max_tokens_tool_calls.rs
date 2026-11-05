//! A streaming Anthropic answer cut short by `max_tokens` in the middle of a
//! `tool_use` block ends with `message_delta` (stop reason `max_tokens`) and
//! `message_stop`, with no `content_block_stop` for the open block. Its tool
//! calls, the last one truncated, are still in the captured end, with the stop
//! reason `MaxTokens`.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};

use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatOptions, ChatRequest, ChatStreamEvent, StopReason, ToolCall};
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

const PARTIAL: &str = r#"{"summary": "The American Rev"#;

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

fn tool_use_start(index: u32, id: &str, name: &str) -> serde_json::Value {
	serde_json::json!({
		"type": "content_block_start",
		"index": index,
		"content_block": { "type": "tool_use", "id": id, "name": name, "input": {} }
	})
}

fn input_delta(index: u32, partial_json: &str) -> serde_json::Value {
	serde_json::json!({
		"type": "content_block_delta",
		"index": index,
		"delta": { "type": "input_json_delta", "partial_json": partial_json }
	})
}

fn block_stop(index: u32) -> serde_json::Value {
	serde_json::json!({ "type": "content_block_stop", "index": index })
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

/// The captured end of a streamed answer: its tool calls and stop reason.
async fn end_of(events: &[serde_json::Value]) -> (Option<Vec<ToolCall>>, Option<StopReason>) {
	let port = serve(sse(events));
	let client = Client::builder().build().expect("client");
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-5-5"),
	};
	let options = ChatOptions::default().with_capture_tool_calls(true);
	let response = client
		.exec_chat_stream(target, ChatRequest::from_user("hi"), Some(&options))
		.await
		.expect("stream");

	let mut stream = response.stream;
	let mut captured = (None, None);
	while let Some(event) = stream.next().await {
		if let ChatStreamEvent::End(end) = event.expect("event") {
			captured = (
				end.captured_tool_calls().map(|calls| calls.into_iter().cloned().collect()),
				end.captured_stop_reason.clone(),
			);
		}
	}
	captured
}

#[tokio::test]
async fn max_tokens_keeps_the_tool_call_it_cut_off() {
	// A complete call, then one the token cap cuts off mid-input, as Anthropic
	// streams it with `eager_input_streaming`: the cut block never stops.
	let mut events = vec![
		message_start(),
		tool_use_start(0, "toolu_1", "think"),
		input_delta(0, r#"{"thought":"#),
		input_delta(0, r#""two differ"}"#),
		block_stop(0),
		tool_use_start(1, "toolu_2", "write_result"),
		input_delta(1, ""),
		input_delta(1, r#"{"summary": "#),
		input_delta(1, r#""The American Rev"#),
	];
	events.extend(message_end("max_tokens"));

	let (calls, stop_reason) = end_of(&events).await;

	let calls = calls.expect("the cut-off answer's tool calls");
	assert_eq!(calls.len(), 2);
	assert_eq!(calls[0].call_id, "toolu_1");
	assert_eq!(calls[0].fn_arguments, serde_json::json!({ "thought": "two differ" }));
	assert_eq!(calls[1].call_id, "toolu_2");
	assert_eq!(calls[1].fn_name, "write_result");
	// Truncated JSON stays the string that streamed.
	assert_eq!(calls[1].fn_arguments, serde_json::Value::String(PARTIAL.to_string()));
	assert!(matches!(stop_reason, Some(StopReason::MaxTokens(_))));
}

#[tokio::test]
async fn max_tokens_keeps_a_tool_call_cut_off_before_its_input() {
	// Without `eager_input_streaming`, Anthropic buffers the input, so a call
	// the cap cuts off streams only an empty delta before the message ends.
	let mut events = vec![
		message_start(),
		tool_use_start(0, "toolu_1", "write_result"),
		input_delta(0, ""),
	];
	events.extend(message_end("max_tokens"));

	let (calls, stop_reason) = end_of(&events).await;

	let calls = calls.expect("the cut-off answer's tool call");
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "toolu_1");
	assert_eq!(calls[0].fn_name, "write_result");
	assert_eq!(calls[0].fn_arguments, serde_json::Value::String(String::new()));
	assert!(matches!(stop_reason, Some(StopReason::MaxTokens(_))));
}

#[tokio::test]
async fn a_finished_tool_call_without_input_has_empty_arguments() {
	let mut events = vec![
		message_start(),
		tool_use_start(0, "toolu_1", "list_files"),
		input_delta(0, ""),
		block_stop(0),
	];
	events.extend(message_end("tool_use"));

	let (calls, stop_reason) = end_of(&events).await;

	let calls = calls.expect("the answer's tool call");
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].fn_arguments, serde_json::json!({}));
	assert!(matches!(stop_reason, Some(StopReason::ToolCall(_))));
}
