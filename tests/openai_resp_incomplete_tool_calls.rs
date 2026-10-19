//! A streaming Responses answer cut short by `max_output_tokens` ends with
//! `response.incomplete`. Its function calls, the last one truncated, are still
//! in the captured end, with the stop reason `MaxTokens`.

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
		body.push_str("data: ");
		body.push_str(&event.to_string());
		body.push_str("\n\n");
	}
	body
}

const PARTIAL: &str = r#"{"summary":"B adds a fall"#;

/// The truncated call, as the terminal `response.incomplete` lists it.
fn truncated_call() -> serde_json::Value {
	serde_json::json!({
		"type": "function_call",
		"status": "incomplete",
		"call_id": "call_2",
		"name": "write_result",
		"arguments": PARTIAL
	})
}

fn incomplete(output: Vec<serde_json::Value>) -> serde_json::Value {
	serde_json::json!({
		"type": "response.incomplete",
		"response": {
			"id": "resp1",
			"status": "incomplete",
			"model": "gpt-5",
			"incomplete_details": { "reason": "max_output_tokens" },
			"output": output,
			"usage": { "input_tokens": 10, "output_tokens": 16 }
		}
	})
}

/// A complete call, then one the token cap cuts off mid-arguments, as OpenAI
/// streams it: the cut call is done with status `incomplete`, and
/// `response.incomplete` lists both.
fn streamed_events() -> Vec<serde_json::Value> {
	let full = r#"{"thought":"two differ"}"#;
	vec![
		serde_json::json!({
			"type": "response.created",
			"response": { "id": "resp1", "status": "in_progress" }
		}),
		serde_json::json!({
			"type": "response.output_item.added",
			"output_index": 0,
			"item": { "type": "function_call", "call_id": "call_1", "name": "think", "arguments": "" }
		}),
		serde_json::json!({
			"type": "response.function_call_arguments.delta",
			"output_index": 0,
			"delta": full
		}),
		serde_json::json!({
			"type": "response.output_item.done",
			"output_index": 0,
			"item": { "type": "function_call", "call_id": "call_1", "name": "think", "arguments": full }
		}),
		serde_json::json!({
			"type": "response.output_item.added",
			"output_index": 1,
			"item": { "type": "function_call", "call_id": "call_2", "name": "write_result", "arguments": "" }
		}),
		serde_json::json!({
			"type": "response.function_call_arguments.delta",
			"output_index": 1,
			"delta": PARTIAL
		}),
		serde_json::json!({
			"type": "response.output_item.done",
			"output_index": 1,
			"item": truncated_call()
		}),
		incomplete(vec![
			serde_json::json!({
				"type": "function_call",
				"status": "completed",
				"call_id": "call_1",
				"name": "think",
				"arguments": full
			}),
			truncated_call(),
		]),
	]
}

/// The captured end of a streamed answer: its tool calls and stop reason.
async fn end_of(events: &[serde_json::Value]) -> (Option<Vec<ToolCall>>, Option<StopReason>) {
	let port = serve(sse(events));
	let client = Client::builder().build().expect("client");
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://127.0.0.1:{port}/v1/")),
		auth: AuthData::Key("test".to_string()),
		model: ModelIden::new(AdapterKind::OpenAIResp, "gpt-5"),
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
async fn incomplete_response_keeps_streamed_tool_calls() {
	let (calls, stop_reason) = end_of(&streamed_events()).await;

	let calls = calls.expect("the incomplete response's tool calls");
	assert_eq!(calls.len(), 2);
	assert_eq!(calls[0].call_id, "call_1");
	assert_eq!(calls[0].fn_arguments, serde_json::json!({ "thought": "two differ" }));
	assert_eq!(calls[1].call_id, "call_2");
	assert_eq!(calls[1].fn_name, "write_result");
	// Truncated JSON stays the string that streamed.
	assert_eq!(calls[1].fn_arguments, serde_json::Value::String(PARTIAL.to_string()));
	assert!(matches!(stop_reason, Some(StopReason::MaxTokens(_))));
}

#[tokio::test]
async fn incomplete_response_falls_back_to_its_output() {
	// Only the terminal event: the calls come from its `output`.
	let (calls, stop_reason) = end_of(&[incomplete(vec![truncated_call()])]).await;

	let calls = calls.expect("the incomplete response's tool calls");
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "call_2");
	assert_eq!(calls[0].fn_arguments, serde_json::Value::String(PARTIAL.to_string()));
	assert!(matches!(stop_reason, Some(StopReason::MaxTokens(_))));
}
