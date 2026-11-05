//! Reproduces a streaming Responses `function_call` where `arguments.delta`
//! is truncated and the complete JSON only appears in `response.output_item.done`,
//! while `response.completed` carries no `output`.

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
		body.push_str("data: ");
		body.push_str(&event.to_string());
		body.push_str("\n\n");
	}
	body
}

/// After a truncated delta, the complete arguments only exist on `done`.
/// `completed` intentionally omits `output`, so the test cannot pass via the
/// `response.output` fallback.
fn partial_delta_events() -> Vec<serde_json::Value> {
	let full = r#"{"command":"ls"}"#;
	vec![
		serde_json::json!({
			"type": "response.created",
			"response": { "id": "resp1", "status": "in_progress" }
		}),
		serde_json::json!({
			"type": "response.output_item.added",
			"output_index": 0,
			"item": {
				"type": "function_call",
				"call_id": "call_1",
				"name": "exec",
				"arguments": ""
			}
		}),
		serde_json::json!({
			"type": "response.function_call_arguments.delta",
			"output_index": 0,
			"delta": "{\"comm"
		}),
		serde_json::json!({
			"type": "response.output_item.done",
			"output_index": 0,
			"item": {
				"type": "function_call",
				"call_id": "call_1",
				"name": "exec",
				"arguments": full
			}
		}),
		serde_json::json!({
			"type": "response.completed",
			"response": { "id": "resp1", "status": "completed", "model": "gpt-5" }
		}),
	]
}

#[tokio::test]
async fn partial_function_call_delta_uses_done_arguments() {
	let port = serve(sse(&partial_delta_events()));
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
	let mut arguments = None;
	while let Some(event) = stream.next().await {
		let event = event.expect("event");
		if let ChatStreamEvent::End(end) = event
			&& let Some(calls) = end.captured_tool_calls()
		{
			arguments = Some(calls[0].fn_arguments.clone());
		}
	}

	assert_eq!(arguments, Some(serde_json::json!({"command": "ls"})));
}
