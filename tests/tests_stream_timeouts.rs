//! Local HTTP tests: transport progress is independent of parsed model events.
use futures::StreamExt;
use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint};
use genai::{Client, ModelIden, ServiceTarget};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const WAIT: Duration = Duration::from_millis(200);

async fn server(mode: &'static str) -> (ServiceTarget, tokio::task::JoinHandle<()>) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let target = ServiceTarget {
		endpoint: Endpoint::from_owned(format!("http://{}/v1/", listener.local_addr().unwrap())),
		auth: AuthData::from_single("test"),
		model: ModelIden::new(AdapterKind::OpenAI, "test"),
	};
	let task = tokio::spawn(async move {
		let (mut socket, _) = listener.accept().await.unwrap();
		let mut request = Vec::new();
		let mut buf = [0; 4096];
		loop {
			let n = socket.read(&mut buf).await.unwrap();
			assert_ne!(n, 0);
			request.extend_from_slice(&buf[..n]);
			if let Some(end) = request.windows(4).position(|s| s == b"\r\n\r\n") {
				let headers = String::from_utf8_lossy(&request[..end]);
				let length: usize = headers
					.lines()
					.find_map(|s| {
						s.to_lowercase()
							.strip_prefix("content-length:")
							.map(|s| s.trim().parse().unwrap())
					})
					.unwrap();
				if request.len() >= end + 4 + length {
					let body: serde_json::Value = serde_json::from_slice(&request[end + 4..end + 4 + length]).unwrap();
					assert!(body.get("stream_header_timeout").is_none());
					assert!(body.get("stream_read_timeout").is_none());
					break;
				}
			}
		}
		if mode == "headers" || mode == "cancel" {
			// The client must release the request on timeout or stream drop.
			assert_eq!(socket.read(&mut buf).await.unwrap(), 0);
			return;
		}
		if mode == "independent" {
			tokio::time::sleep(Duration::from_millis(350)).await;
		}
		let status = if mode == "error-body" {
			"429 Too Many Requests"
		} else {
			"200 OK"
		};
		socket
			.write_all(
				format!(
					"HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nRetry-After: 2\r\nConnection: close\r\n\r\n"
				)
				.as_bytes(),
			)
			.await
			.unwrap();
		if mode == "body" || mode == "error-body" {
			assert_eq!(socket.read(&mut buf).await.unwrap(), 0);
			return;
		}
		if mode == "heartbeat" {
			for _ in 0..8 {
				socket.write_all(b": ping\n\n").await.unwrap();
				tokio::time::sleep(Duration::from_millis(60)).await;
			}
		}
		if mode == "fragment" {
			for part in [
				"data: ",
				"{\"choices\":",
				"[{\"index\":0,",
				"\"delta\":",
				"{\"content\":",
				"\"hello\"}}]}",
				"\n\n",
			] {
				socket.write_all(part.as_bytes()).await.unwrap();
				tokio::time::sleep(Duration::from_millis(60)).await;
			}
		}
		socket.write_all(b"data: [DONE]\n\n").await.unwrap();
	});
	(target, task)
}

fn options() -> ChatOptions {
	ChatOptions::default()
		.with_stream_header_timeout(WAIT)
		.with_stream_read_timeout(WAIT)
}

async fn run(mode: &'static str, defaults: ChatOptions, per_call: Option<ChatOptions>) -> Option<genai::Error> {
	let (target, task) = server(mode).await;
	let client = Client::builder().with_chat_options(defaults).build().unwrap();
	let mut response = client
		.exec_chat_stream(
			target,
			ChatRequest::new(vec![ChatMessage::user("hello")]),
			per_call.as_ref(),
		)
		.await
		.unwrap();
	let error = tokio::time::timeout(Duration::from_secs(5), async {
		while let Some(event) = response.stream.next().await {
			if let Err(error) = event {
				return Some(error);
			}
		}
		None
	})
	.await
	.expect("stream must finish");
	drop(response);
	tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap();
	error
}

#[tokio::test]
async fn header_timeout_releases_request() {
	let error = run("headers", options(), None).await.unwrap();
	assert!(error.to_string().contains("headers timed out"), "{error}");
}

#[tokio::test]
async fn idle_body_timeout_releases_response() {
	let error = run("body", options(), None).await.unwrap();
	assert!(error.to_string().contains("body read timed out"), "{error}");
}

#[tokio::test]
async fn stalled_error_body_preserves_http_status_and_retry_headers() {
	let error = run("error-body", options(), None).await.unwrap();
	let genai::Error::WebStream { error, .. } = error else {
		panic!("expected stream error")
	};
	let error = error.downcast::<genai::Error>().expect("HTTP error remains inspectable");
	assert_eq!(error.status().unwrap().as_u16(), 429);
	assert_eq!(error.headers().unwrap()["retry-after"], "2");
	assert!(error.to_string().contains("body read timed out"), "{error}");
}

#[tokio::test]
async fn heartbeats_reset_read_timeout_without_model_events() {
	assert!(run("heartbeat", options(), None).await.is_none());
}

#[tokio::test]
async fn partial_frames_reset_read_timeout() {
	assert!(run("fragment", options(), None).await.is_none());
}

#[tokio::test]
async fn per_call_header_override_does_not_consume_read_budget() {
	let override_options = ChatOptions {
		stream_header_timeout: Some(Duration::from_secs(2)),
		..Default::default()
	};
	assert!(run("independent", options(), Some(override_options)).await.is_none());
}

#[tokio::test]
async fn unset_transport_timeouts_preserve_existing_behavior() {
	assert!(run("independent", ChatOptions::default(), None).await.is_none());
}

#[test]
fn transport_options_are_not_serialized() {
	let value = serde_json::to_value(options()).unwrap();
	assert!(value.get("stream_header_timeout").is_none());
	assert!(value.get("stream_read_timeout").is_none());
}

#[tokio::test]
async fn per_call_read_timeout_overrides_client_default() {
	let defaults = ChatOptions {
		stream_read_timeout: Some(Duration::from_millis(1)),
		..options()
	};
	let per_call = ChatOptions {
		stream_read_timeout: Some(WAIT),
		..Default::default()
	};
	assert!(run("heartbeat", defaults, Some(per_call)).await.is_none());
}

#[tokio::test]
async fn dropping_stream_cancels_pending_http_request() {
	let (target, task) = server("cancel").await;
	let client = Client::builder().build().unwrap();
	let mut response = client
		.exec_chat_stream(target, ChatRequest::new(vec![ChatMessage::user("hello")]), None)
		.await
		.unwrap();
	// Poll past the synthetic Start into the pending HTTP send.
	assert!(
		tokio::time::timeout(WAIT, async {
			while let Some(event) = response.stream.next().await {
				event.unwrap();
			}
		})
		.await
		.is_err()
	);
	drop(response);
	tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap();
}

#[tokio::test]
async fn reqwest_total_timeout_remains_in_effect() {
	let (target, task) = server("body").await;
	let http = reqwest::Client::builder().timeout(WAIT).build().unwrap();
	let client = Client::builder().with_reqwest(http).build().unwrap();
	let long_timeout = ChatOptions {
		stream_header_timeout: Some(Duration::from_secs(10)),
		stream_read_timeout: Some(Duration::from_secs(10)),
		..Default::default()
	};
	let mut response = client
		.exec_chat_stream(
			target,
			ChatRequest::new(vec![ChatMessage::user("hello")]),
			Some(&long_timeout),
		)
		.await
		.unwrap();
	let error = tokio::time::timeout(Duration::from_secs(2), async {
		while let Some(event) = response.stream.next().await {
			if let Err(error) = event {
				return error;
			}
		}
		panic!("expected reqwest timeout");
	})
	.await
	.unwrap();
	let genai::Error::WebStream { error, .. } = error else {
		panic!("expected stream error")
	};
	assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_timeout());
	drop(response);
	tokio::time::timeout(Duration::from_secs(2), task).await.unwrap().unwrap();
}
