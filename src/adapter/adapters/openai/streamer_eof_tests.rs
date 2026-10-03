//! Offline regression tests using synthetic SSE over a local HTTP connection.
use super::*;
use crate::chat::{ChatOptions, ChatStream, ChatStreamEvent, StreamEnd};
use futures::StreamExt;
use serde_json::json;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn collect(body: String, fragmented: bool, truncated_http: bool, capture: bool) -> Result<Vec<ChatStreamEvent>> {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let server = tokio::spawn(async move {
		let (mut socket, _) = listener.accept().await.unwrap();
		let mut request = Vec::new();
		let mut buf = [0; 4096];
		while !request.windows(4).any(|w| w == b"\r\n\r\n") {
			let n = socket.read(&mut buf).await.unwrap();
			assert!(n > 0);
			request.extend_from_slice(&buf[..n]);
		}
		let extra_header = if truncated_http {
			format!("Content-Length: {}\r\n", body.len() + 100)
		} else {
			String::new()
		};
		socket
			.write_all(
				format!(
					"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n{extra_header}Connection: close\r\n\r\n"
				)
				.as_bytes(),
			)
			.await
			.unwrap();
		let size = if fragmented { 1 } else { body.len().max(1) };
		for part in body.as_bytes().chunks(size) {
			if socket.write_all(part).await.is_err() {
				return;
			}
			if fragmented {
				tokio::time::sleep(Duration::from_millis(1)).await;
			}
		}
		socket.shutdown().await.unwrap();
	});
	let client = reqwest::Client::builder().no_proxy().build().unwrap();
	let options = ChatOptions::default()
		.with_capture_content(capture)
		.with_capture_reasoning_content(capture)
		.with_capture_tool_calls(capture)
		.with_capture_usage(capture);
	let streamer = OpenAIStreamer::new(
		EventSourceStream::new(client.get(format!("http://{addr}/"))),
		ModelIden::new(AdapterKind::OpenAI, "test"),
		ChatOptionsSet::default().with_chat_options(Some(&options)),
	);
	let mut stream = ChatStream::from_inter_stream(streamer);
	let result = tokio::time::timeout(Duration::from_secs(5), async {
		let mut events = Vec::new();
		while let Some(event) = stream.next().await {
			events.push(event?);
		}
		// Polling a completed stream again must not emit another End.
		assert!(stream.next().await.is_none());
		Ok(events)
	})
	.await
	.unwrap();
	server.await.unwrap();
	result
}
fn frame(delta: Value, reason: Option<&str>) -> String {
	format!(
		"data: {}\n\n",
		json!({"choices":[{"index":0,"delta":delta,"finish_reason":reason}]})
	)
}
fn finish(reason: &str) -> String {
	frame(json!({}), Some(reason))
}
fn text() -> String {
	frame(json!({"content":"你好"}), None)
}
fn tool(args: &str) -> String {
	frame(
		json!({"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"get_weather","arguments":args}}]}),
		None,
	)
}
fn one_end(events: &[ChatStreamEvent]) -> &StreamEnd {
	let ends: Vec<_> = events
		.iter()
		.filter_map(|e| match e {
			ChatStreamEvent::End(end) => Some(end),
			_ => None,
		})
		.collect();
	assert_eq!(ends.len(), 1, "expected exactly one End");
	ends[0]
}
#[tokio::test]
async fn clean_eof_after_finish_preserves_text_reasoning_and_usage_tail() {
	for done in ["", "data: [DONE]\n\n"] {
		let body = format!(
			"{}{}{}data: {{\"choices\":[],\"usage\":{{\"prompt_tokens\":10,\"completion_tokens\":20,\"total_tokens\":30}}}}\n\n{done}",
			frame(json!({"reasoning_content":"thinking"}), None),
			text(),
			finish("stop")
		);
		let events = collect(body, false, false, true).await.unwrap();
		let end = one_end(&events);
		assert_eq!(end.captured_stop_reason, Some(StopReason::Completed("stop".into())));
		assert_eq!(end.captured_content.as_ref().unwrap().texts().join(""), "你好");
		assert_eq!(end.captured_reasoning_content.as_deref(), Some("thinking"));
		assert_eq!(end.captured_usage.as_ref().unwrap().total_tokens, Some(30));
	}
}
#[tokio::test]
async fn clean_eof_after_tool_completion_parses_arguments() {
	let args = json!({"city":"Paris"});
	let events = collect(
		format!("{}{}", tool(&args.to_string()), finish("tool_calls")),
		false,
		false,
		true,
	)
	.await
	.unwrap();
	let end = one_end(&events);
	let calls = end.captured_tool_calls().unwrap();
	assert_eq!(calls.len(), 1);
	assert_eq!(calls[0].call_id, "call_1");
	assert_eq!(calls[0].fn_arguments, args);
	assert_eq!(
		end.captured_stop_reason,
		Some(StopReason::ToolCall("tool_calls".into()))
	);
}
#[tokio::test]
async fn eof_preserves_length_and_filter_reasons_with_incomplete_arguments() {
	for reason in ["length", "content_filter"] {
		let events = collect(format!("{}{}", tool("{"), finish(reason)), false, false, true)
			.await
			.unwrap();
		assert_eq!(one_end(&events).captured_stop_reason.as_ref().unwrap().raw(), reason);
	}
}
#[tokio::test]
async fn eof_without_recognized_finish_reason_never_emits_end() {
	for reason in [None, Some(""), Some("unknown")] {
		let body = format!("{}{}", text(), frame(json!({}), reason));
		let events = collect(body, false, false, true).await.unwrap();
		assert!(!events.iter().any(|e| matches!(e, ChatStreamEvent::End(_))));
	}
}
#[tokio::test]
async fn eof_does_not_accept_incomplete_tool_calls_as_success() {
	for body in [
		format!("{}{}", tool("{"), finish("tool_calls")),
		format!("{}{}", tool("{"), finish("stop")),
		finish("tool_calls"),
	] {
		assert!(collect(body, false, false, true).await.is_err());
	}
}
#[tokio::test]
async fn provider_error_after_finish_reason_is_not_swallowed() {
	let body = format!(
		"{}{}data: {{\"error\":{{\"message\":\"upstream failed\"}}}}\n\n",
		text(),
		finish("stop")
	);
	assert!(collect(body, false, false, true).await.is_err());
}
#[tokio::test]
async fn transport_error_after_finish_reason_is_not_swallowed() {
	assert!(
		collect(format!("{}{}", text(), finish("stop")), false, true, true)
			.await
			.is_err()
	);
}
#[tokio::test]
async fn capture_disabled_still_emits_terminal_reason() {
	let events = collect(format!("{}{}", tool("{}"), finish("tool_calls")), false, false, false)
		.await
		.unwrap();
	let end = one_end(&events);
	assert_eq!(
		end.captured_stop_reason,
		Some(StopReason::ToolCall("tool_calls".into()))
	);
	assert!(end.captured_content.is_none());
	assert!(end.captured_usage.is_none());
}
#[tokio::test]
async fn fragmented_utf8_and_done_marker_still_complete_once() {
	let body = format!("{}{}data: [DONE]\r\n\r\n", text(), finish("stop"));
	let events = collect(body, true, false, true).await.unwrap();
	assert_eq!(
		one_end(&events).captured_content.as_ref().unwrap().texts().join(""),
		"你好"
	);
}
