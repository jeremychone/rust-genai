//! Offline tests for the per-request exec hooks (`PayloadInterceptor` / `ResponseObserver`)
//! on the chat exec paths (`exec_chat` and `exec_chat_stream`), using a local one-shot
//! HTTP server (no network, no provider keys).

use crate::adapter::AdapterKind;
use crate::chat::{ChatRequest, ChatStreamEvent};
use crate::resolver::{AuthData, Endpoint};
use crate::{Client, Error, ModelIden, ResponseObserver, ServiceTarget};
use futures::StreamExt;
use reqwest::StatusCode;
use reqwest::header::HeaderMap;
use serde_json::{Value, json};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_client_exec_chat_stream_payload_interceptor_replaces_wire_payload() -> Result<()> {
	// -- Setup & Fixtures
	let (url, body_rx) = support_spawn_capture_server(support_sse_ok_response()).await?;
	let seen: Arc<Mutex<Option<(ModelIden, Value)>>> = Arc::new(Mutex::new(None));
	let seen_clone = seen.clone();
	let client = Client::builder()
		.with_payload_interceptor_fn(move |model_iden: ModelIden, mut payload: Value| -> Option<Value> {
			*seen_clone.lock().unwrap() = Some((model_iden, payload.clone()));
			payload["x_intercepted"] = json!(true);
			Some(payload)
		})
		.build();

	// -- Exec
	let chat_res = client
		.exec_chat_stream(
			support_target(&url),
			ChatRequest::from_user("Why is the sky red?"),
			None,
		)
		.await?;
	let content = support_collect_content(chat_res).await?;

	// -- Check
	assert_eq!(content, "Hello");
	// The interceptor saw the target ModelIden and the original serialized payload.
	let (seen_model, seen_payload) = seen.lock().unwrap().take().ok_or("Interceptor should have been called")?;
	assert_eq!(seen_model.adapter_kind, AdapterKind::OpenAI);
	assert_eq!(&*seen_model.model_name, "gpt-test");
	assert_eq!(seen_payload.get("model").and_then(|v| v.as_str()), Some("gpt-test"));
	assert_eq!(seen_payload.get("x_intercepted"), None);
	// The replacement payload is what actually went over the wire.
	let wire_body = body_rx.await?;
	let wire_json: Value = serde_json::from_str(&wire_body)?;
	assert_eq!(wire_json.get("x_intercepted"), Some(&json!(true)));
	assert_eq!(wire_json.get("model").and_then(|v| v.as_str()), Some("gpt-test"));

	Ok(())
}

#[tokio::test]
async fn test_client_exec_chat_stream_response_observer_on_success() -> Result<()> {
	// -- Setup & Fixtures
	let (url, _body_rx) = support_spawn_capture_server(support_sse_ok_response()).await?;
	let (observed, order) = support_new_observer_state();
	let client = Client::builder()
		.with_response_observer(support_async_observer(observed.clone(), order.clone()))
		.build();

	// -- Exec
	let mut chat_res = client
		.exec_chat_stream(
			support_target(&url),
			ChatRequest::from_user("Why is the sky red?"),
			None,
		)
		.await?;
	// The HTTP send is lazy — nothing observed before the stream is polled.
	assert!(
		observed.lock().unwrap().is_none(),
		"Observer should not fire before first poll"
	);
	let mut content = String::new();
	while let Some(event) = chat_res.stream.next().await {
		if let ChatStreamEvent::Chunk(chunk) = event? {
			if content.is_empty() {
				order.lock().unwrap().push("first-chunk".to_string());
			}
			content.push_str(&chunk.content);
		}
	}

	// -- Check
	assert_eq!(content, "Hello");
	let (model_iden, status, headers) = observed.lock().unwrap().take().ok_or("Observer should have fired")?;
	assert_eq!(&*model_iden.model_name, "gpt-test");
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		headers.get("x-obs-test").and_then(|v| v.to_str().ok()),
		Some("obs-value")
	);
	// The observer fired on the response head, before the stream body was consumed.
	assert_eq!(
		*order.lock().unwrap(),
		vec!["observer".to_string(), "first-chunk".to_string()]
	);

	Ok(())
}

#[tokio::test]
async fn test_client_exec_chat_stream_response_observer_on_http_error() -> Result<()> {
	// -- Setup & Fixtures
	let error_body = r#"{"error":{"message":"rate limited"}}"#;
	let raw_response = format!(
		"HTTP/1.1 429 Too Many Requests\r\n\
		content-type: application/json\r\n\
		retry-after: 2\r\n\
		content-length: {}\r\n\
		connection: close\r\n\
		\r\n\
		{error_body}",
		error_body.len()
	);
	let (url, _body_rx) = support_spawn_capture_server(raw_response).await?;
	let (observed, _order) = support_new_observer_state();
	let client = Client::builder()
		.with_response_observer(support_async_observer(observed.clone(), _order.clone()))
		.build();

	// -- Exec
	let mut chat_res = client
		.exec_chat_stream(
			support_target(&url),
			ChatRequest::from_user("Why is the sky red?"),
			None,
		)
		.await?;
	let mut stream_err: Option<Error> = None;
	while let Some(event) = chat_res.stream.next().await {
		if let Err(err) = event {
			stream_err = Some(err);
			break;
		}
	}

	// -- Check
	// The observer fired on the failing response head.
	let (_model_iden, status, headers) = observed.lock().unwrap().take().ok_or("Observer should have fired")?;
	assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
	assert_eq!(headers.get("retry-after").and_then(|v| v.to_str().ok()), Some("2"));
	// AND the returned HttpError still carries the response headers (block-1 behavior).
	let stream_err = stream_err.ok_or("Stream should have yielded an error")?;
	let Error::WebStream { error, .. } = stream_err else {
		return Err(format!("Should be Error::WebStream, but was: {stream_err}").into());
	};
	let http_err = error
		.downcast::<Error>()
		.map_err(|err| format!("Error should downcast to genai Error, but was: {err}"))?;
	match *http_err {
		Error::HttpError {
			status, body, headers, ..
		} => {
			assert_eq!(status.as_u16(), 429);
			assert_eq!(body, error_body);
			assert_eq!(headers.get("retry-after").and_then(|v| v.to_str().ok()), Some("2"));
		}
		other => return Err(format!("Should be Error::HttpError, but was: {other}").into()),
	}

	Ok(())
}

#[tokio::test]
async fn test_client_exec_chat_stream_no_hooks_regression() -> Result<()> {
	// -- Setup & Fixtures
	let (url_baseline, body_rx_baseline) = support_spawn_capture_server(support_sse_ok_response()).await?;
	let (url_noop, body_rx_noop) = support_spawn_capture_server(support_sse_ok_response()).await?;
	let client_baseline = Client::builder().build();
	// A `None`-returning interceptor must keep the payload unchanged (byte-identical wire body).
	let client_noop = Client::builder()
		.with_payload_interceptor_fn(|_model_iden: ModelIden, _payload: Value| -> Option<Value> { None })
		.build();

	// -- Exec
	let chat_req = ChatRequest::from_user("Why is the sky red?");
	let res_baseline = client_baseline
		.exec_chat_stream(support_target(&url_baseline), chat_req.clone(), None)
		.await?;
	let content_baseline = support_collect_content(res_baseline).await?;
	let res_noop = client_noop.exec_chat_stream(support_target(&url_noop), chat_req, None).await?;
	let content_noop = support_collect_content(res_noop).await?;

	// -- Check
	assert_eq!(content_baseline, "Hello");
	assert_eq!(content_noop, "Hello");
	let body_baseline = body_rx_baseline.await?;
	let body_noop = body_rx_noop.await?;
	assert_eq!(body_baseline, body_noop, "Wire payload must be byte-identical");

	Ok(())
}

#[tokio::test]
async fn test_client_exec_chat_payload_interceptor_and_observer() -> Result<()> {
	// -- Setup & Fixtures
	let (url, body_rx) = support_spawn_capture_server(support_json_ok_response()).await?;
	let (observed, _order) = support_new_observer_state();
	let client = Client::builder()
		.with_payload_interceptor_fn(|_model_iden: ModelIden, mut payload: Value| -> Option<Value> {
			payload["x_intercepted"] = json!(true);
			Some(payload)
		})
		.with_response_observer(support_async_observer(observed.clone(), _order.clone()))
		.build();

	// -- Exec
	let chat_res = client
		.exec_chat(
			support_target(&url),
			ChatRequest::from_user("Why is the sky red?"),
			None,
		)
		.await?;

	// -- Check
	assert_eq!(chat_res.first_text(), Some("Hello"));
	let wire_body = body_rx.await?;
	let wire_json: Value = serde_json::from_str(&wire_body)?;
	assert_eq!(wire_json.get("x_intercepted"), Some(&json!(true)));
	let (model_iden, status, headers) = observed.lock().unwrap().take().ok_or("Observer should have fired")?;
	assert_eq!(&*model_iden.model_name, "gpt-test");
	assert_eq!(status, StatusCode::OK);
	assert_eq!(
		headers.get("x-obs-test").and_then(|v| v.to_str().ok()),
		Some("obs-value")
	);

	Ok(())
}

#[tokio::test]
async fn test_client_exec_chat_response_observer_on_http_error() -> Result<()> {
	// -- Setup & Fixtures
	let error_body = r#"{"error":{"message":"boom"}}"#;
	let raw_response = format!(
		"HTTP/1.1 500 Internal Server Error\r\n\
		content-type: application/json\r\n\
		x-obs-test: obs-value\r\n\
		content-length: {}\r\n\
		connection: close\r\n\
		\r\n\
		{error_body}",
		error_body.len()
	);
	let (url, _body_rx) = support_spawn_capture_server(raw_response).await?;
	let (observed, _order) = support_new_observer_state();
	let client = Client::builder()
		.with_response_observer(support_async_observer(observed.clone(), _order.clone()))
		.build();

	// -- Exec
	let res = client
		.exec_chat(
			support_target(&url),
			ChatRequest::from_user("Why is the sky red?"),
			None,
		)
		.await;

	// -- Check
	// The observer fired on the failing response head, before the error body was consumed.
	let (_model_iden, status, headers) = observed.lock().unwrap().take().ok_or("Observer should have fired")?;
	assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
	assert_eq!(
		headers.get("x-obs-test").and_then(|v| v.to_str().ok()),
		Some("obs-value")
	);
	// And the call still returns the regular web error.
	let err = res.err().ok_or("exec_chat should have failed")?;
	let Error::WebModelCall { webc_error, .. } = err else {
		return Err(format!("Should be Error::WebModelCall, but was: {err}").into());
	};
	match webc_error {
		crate::webc::Error::ResponseFailedStatus { status, body, .. } => {
			assert_eq!(status.as_u16(), 500);
			assert_eq!(body, error_body);
		}
		other => return Err(format!("Should be ResponseFailedStatus, but was: {other}").into()),
	}

	Ok(())
}

// region:    --- Support

/// Builds a fully-resolved ServiceTarget pointing at the local test server (OpenAI adapter).
fn support_target(url: &str) -> ServiceTarget {
	ServiceTarget {
		endpoint: Endpoint::from_owned(url.to_string()),
		auth: AuthData::from_single("test-key"),
		model: ModelIden::new(AdapterKind::OpenAI, "gpt-test"),
	}
}

/// Raw SSE success response with one content chunk (OpenAI chat completions shape).
fn support_sse_ok_response() -> String {
	let chunk = r#"{"id":"chatcmpl-1","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
	format!(
		"HTTP/1.1 200 OK\r\n\
		content-type: text/event-stream\r\n\
		x-obs-test: obs-value\r\n\
		connection: close\r\n\
		\r\n\
		data: {chunk}\n\ndata: [DONE]\n\n"
	)
}

/// Raw JSON success response (OpenAI chat completions shape) for the non-streaming path.
fn support_json_ok_response() -> String {
	let body = r#"{"id":"chatcmpl-1","model":"gpt-test","choices":[{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
	format!(
		"HTTP/1.1 200 OK\r\n\
		content-type: application/json\r\n\
		x-obs-test: obs-value\r\n\
		content-length: {}\r\n\
		connection: close\r\n\
		\r\n\
		{body}",
		body.len()
	)
}

type ObservedState = Arc<Mutex<Option<(ModelIden, StatusCode, HeaderMap)>>>;
type OrderState = Arc<Mutex<Vec<String>>>;

fn support_new_observer_state() -> (ObservedState, OrderState) {
	(Arc::new(Mutex::new(None)), Arc::new(Mutex::new(Vec::new())))
}

/// Builds an async ResponseObserver that records the observed (model, status, headers) and
/// appends "observer" to the order log (to assert it fired before body consumption).
fn support_async_observer(observed: ObservedState, order: OrderState) -> ResponseObserver {
	ResponseObserver::from_observer_async_fn(
		move |model_iden: ModelIden,
		      status: StatusCode,
		      headers: HeaderMap|
		      -> Pin<Box<dyn Future<Output = ()> + Send>> {
			let observed = observed.clone();
			let order = order.clone();
			Box::pin(async move {
				*observed.lock().unwrap() = Some((model_iden, status, headers));
				order.lock().unwrap().push("observer".to_string());
			})
		},
	)
}

/// Consumes the chat stream and concatenates the text chunks.
async fn support_collect_content(mut chat_res: crate::chat::ChatStreamResponse) -> Result<String> {
	let mut content = String::new();
	while let Some(event) = chat_res.stream.next().await {
		if let ChatStreamEvent::Chunk(chunk) = event? {
			content.push_str(&chunk.content);
		}
	}
	Ok(content)
}

/// Spawns a one-shot HTTP server that reads the full request (headers + content-length body),
/// sends the captured request body through the returned channel, then answers with the given
/// raw HTTP response.
async fn support_spawn_capture_server(
	raw_response: String,
) -> Result<(String, tokio::sync::oneshot::Receiver<String>)> {
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	let (body_tx, body_rx) = tokio::sync::oneshot::channel::<String>();
	tokio::spawn(async move {
		if let Ok((mut socket, _)) = listener.accept().await {
			// -- Read the full request: headers, then content-length body bytes.
			let mut buf: Vec<u8> = Vec::new();
			let mut chunk = [0u8; 4096];
			let body = loop {
				let Ok(n) = socket.read(&mut chunk).await else {
					break String::new();
				};
				if n == 0 {
					break String::new();
				}
				buf.extend_from_slice(&chunk[..n]);
				if let Some(header_end) = support_find_subslice(&buf, b"\r\n\r\n") {
					let headers_txt = String::from_utf8_lossy(&buf[..header_end]).to_lowercase();
					let content_length: usize = headers_txt
						.lines()
						.find_map(|line| line.strip_prefix("content-length:"))
						.and_then(|v| v.trim().parse().ok())
						.unwrap_or(0);
					let body_start = header_end + 4;
					if buf.len() >= body_start + content_length {
						break String::from_utf8_lossy(&buf[body_start..body_start + content_length]).to_string();
					}
				}
			};
			let _ = body_tx.send(body);
			let _ = socket.write_all(raw_response.as_bytes()).await;
			let _ = socket.shutdown().await;
		}
	});
	Ok((format!("http://{addr}/"), body_rx))
}

fn support_find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
	haystack.windows(needle.len()).position(|window| window == needle)
}

// endregion: --- Support
