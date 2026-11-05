//! A streamed Anthropic answer can carry `redacted_thinking` blocks among its signed
//! `thinking` blocks. Each arrives whole in its `content_block_start` (`data`, no deltas).
//! The captured content keeps it, in its place, as the custom part a non-streamed answer
//! makes of it, and the next request sends every thinking block back in the order it came.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::mpsc;

use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest, ChatStreamEvent, ContentPart, MessageContent, ToolResponse};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ServiceTarget};
use serde_json::{Value, json};

/// Answers one request with `body` as an event stream and hands back the request's body.
fn serve(body: String) -> (u16, mpsc::Receiver<Value>) {
	let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
	let port = listener.local_addr().expect("addr").port();
	let (sender, receiver) = mpsc::channel();
	std::thread::spawn(move || {
		let (mut stream, _) = listener.accept().expect("accept");
		let mut request = Vec::new();
		let mut buf = [0_u8; 8192];
		let body_start = loop {
			let read = stream.read(&mut buf).expect("read");
			request.extend_from_slice(&buf[..read]);
			if let Some(at) = request.windows(4).position(|window| window == b"\r\n\r\n") {
				break at + 4;
			}
		};
		let head = String::from_utf8_lossy(&request[..body_start]).to_lowercase();
		let length: usize = head
			.lines()
			.find_map(|line| line.strip_prefix("content-length:"))
			.and_then(|value| value.trim().parse().ok())
			.expect("content-length");
		while request.len() < body_start + length {
			let read = stream.read(&mut buf).expect("read");
			request.extend_from_slice(&buf[..read]);
		}
		let _ = sender.send(serde_json::from_slice(&request[body_start..body_start + length]).expect("json body"));
		let header = format!(
			"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
			body.len()
		);
		stream.write_all(header.as_bytes()).expect("header");
		stream.write_all(body.as_bytes()).expect("body");
		let _ = stream.shutdown(Shutdown::Both);
	});
	(port, receiver)
}

fn sse(events: &[Value]) -> String {
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

fn message_start() -> Value {
	json!({
		"type": "message_start",
		"message": {
			"id": "msg_1",
			"type": "message",
			"role": "assistant",
			"model": "claude-opus-4-8",
			"content": [],
			"stop_reason": null,
			"usage": { "input_tokens": 478, "output_tokens": 16 }
		}
	})
}

fn thinking(index: u32, text: &str, signature: &str) -> Vec<Value> {
	vec![
		json!({
			"type": "content_block_start",
			"index": index,
			"content_block": { "type": "thinking", "thinking": "", "signature": "" }
		}),
		json!({
			"type": "content_block_delta",
			"index": index,
			"delta": { "type": "thinking_delta", "thinking": text }
		}),
		json!({
			"type": "content_block_delta",
			"index": index,
			"delta": { "type": "signature_delta", "signature": signature }
		}),
		json!({ "type": "content_block_stop", "index": index }),
	]
}

fn redacted_thinking(index: u32, data: &str) -> Vec<Value> {
	vec![
		json!({
			"type": "content_block_start",
			"index": index,
			"content_block": { "type": "redacted_thinking", "data": data }
		}),
		json!({ "type": "content_block_stop", "index": index }),
	]
}

fn tool_use(index: u32) -> Vec<Value> {
	vec![
		json!({
			"type": "content_block_start",
			"index": index,
			"content_block": { "type": "tool_use", "id": "toolu_1", "name": "get_weather", "input": {} }
		}),
		json!({
			"type": "content_block_delta",
			"index": index,
			"delta": { "type": "input_json_delta", "partial_json": r#"{"city": "Paris"}"# }
		}),
		json!({ "type": "content_block_stop", "index": index }),
	]
}

fn message_end(stop_reason: &str) -> Vec<Value> {
	vec![
		json!({
			"type": "message_delta",
			"delta": { "stop_reason": stop_reason, "stop_sequence": null },
			"usage": { "output_tokens": 120 }
		}),
		json!({ "type": "message_stop" }),
	]
}

fn target(port: u16) -> ServiceTarget {
	ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::Anthropic, "claude-opus-4-8"),
	}
}

/// Streams `events` as the answer to `request`, and returns the captured content of the
/// answer with the body of the request that was sent.
async fn stream(request: ChatRequest, events: &[Value]) -> (Option<MessageContent>, Value) {
	let (port, sent) = serve(sse(events));
	let client = Client::builder().build().expect("client");
	let options = ChatOptions::default()
		.with_capture_content(true)
		.with_capture_reasoning_content(true)
		.with_capture_tool_calls(true);
	let response = client
		.exec_chat_stream(target(port), request, Some(&options))
		.await
		.expect("stream");

	let mut stream = response.stream;
	let mut captured = None;
	while let Some(event) = stream.next().await {
		if let ChatStreamEvent::End(end) = event.expect("event") {
			captured = end.captured_content;
		}
	}
	(captured, sent.recv().expect("request body"))
}

/// A signed block, a redacted one, another signed one, then the tool call.
fn answer() -> Vec<Value> {
	let mut events = vec![message_start()];
	events.extend(thinking(0, "First, the city.", "sig-1"));
	events.extend(redacted_thinking(1, "redacted-blob"));
	events.extend(thinking(2, "Then the weather.", "sig-2"));
	events.extend(tool_use(3));
	events.extend(message_end("tool_use"));
	events
}

#[tokio::test]
async fn a_streamed_redacted_thinking_block_is_kept_in_its_place() {
	let (content, _) = stream(ChatRequest::from_user("Weather in Paris?"), &answer()).await;

	let parts: Vec<Value> = content
		.expect("captured content")
		.into_iter()
		.map(|part| match part {
			ContentPart::ThoughtSignature(signature) => json!({ "signature": signature }),
			ContentPart::ReasoningContent(reasoning) => json!({ "reasoning": reasoning }),
			ContentPart::Custom(custom) => json!({ "custom": custom.data }),
			ContentPart::ToolCall(call) => json!({ "tool_call": call.call_id }),
			other => json!({ "other": format!("{other:?}") }),
		})
		.collect();
	assert_eq!(
		parts,
		[
			json!({ "signature": "sig-1" }),
			json!({ "reasoning": "First, the city." }),
			json!({ "custom": { "type": "redacted_thinking", "data": "redacted-blob" } }),
			json!({ "signature": "sig-2" }),
			json!({ "reasoning": "Then the weather." }),
			json!({ "tool_call": "toolu_1" }),
		]
	);
}

#[tokio::test]
async fn the_next_request_sends_the_thinking_blocks_back_in_their_order() {
	let (content, _) = stream(ChatRequest::from_user("Weather in Paris?"), &answer()).await;
	let next = ChatRequest::new(vec![
		ChatMessage::user("Weather in Paris?"),
		ChatMessage::assistant(content.expect("captured content")),
		ChatMessage::from(ToolResponse::new("toolu_1", "Sunny, 21C")),
	]);

	let mut final_answer = vec![message_start()];
	final_answer.extend(message_end("end_turn"));
	let (_, sent) = stream(next, &final_answer).await;

	assert_eq!(
		sent["messages"][1]["content"],
		json!([
			{ "type": "thinking", "thinking": "First, the city.", "signature": "sig-1" },
			{ "type": "redacted_thinking", "data": "redacted-blob" },
			{ "type": "thinking", "thinking": "Then the weather.", "signature": "sig-2" },
			{ "type": "tool_use", "id": "toolu_1", "name": "get_weather", "input": { "city": "Paris" } },
		])
	);
}
