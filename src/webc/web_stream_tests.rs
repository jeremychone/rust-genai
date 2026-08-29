use super::*;
use crate::ModelIden;
use crate::adapter::AdapterKind;
use crate::chat::{ChatFrameSink, CollectorSink, FrameCtx};
use crate::error::Error as GenaiError;
use crate::webc::{Event, EventSourceStream, FrameTap, Message};
use futures::StreamExt;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_web_stream_http_error_captures_headers() -> Result<()> {
	// -- Setup & Fixtures
	let body = r#"{"error":{"message":"rate limited"}}"#;
	let raw_response = format!(
		"HTTP/1.1 429 Too Many Requests\r\n\
		content-type: application/json\r\n\
		retry-after: 2\r\n\
		retry-after-ms: 1500\r\n\
		x-should-retry: true\r\n\
		content-length: {}\r\n\
		connection: close\r\n\
		\r\n\
		{body}",
		body.len()
	);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let reqwest_builder = reqwest::Client::new().post(&url).json(&serde_json::json!({"stream": true}));
	let mut web_stream = WebStream::new_with_sse(reqwest_builder);

	// -- Exec
	let first_item = web_stream.next().await.ok_or("Should have a first stream item")?;

	// -- Check
	let err = first_item.err().ok_or("First stream item should be an error")?;
	let err = err
		.downcast::<GenaiError>()
		.map_err(|err| format!("Error should downcast to genai Error, but was: {err}"))?;
	match *err {
		GenaiError::HttpError {
			status,
			canonical_reason,
			body: err_body,
			headers,
		} => {
			assert_eq!(status.as_u16(), 429);
			assert_eq!(canonical_reason, "Too Many Requests");
			assert_eq!(err_body, body);
			assert_eq!(headers.get("retry-after").and_then(|v| v.to_str().ok()), Some("2"));
			assert_eq!(
				headers.get("retry-after-ms").and_then(|v| v.to_str().ok()),
				Some("1500")
			);
			assert_eq!(
				headers.get("x-should-retry").and_then(|v| v.to_str().ok()),
				Some("true")
			);
		}
		other => return Err(format!("Should be an Error::HttpError, but was: {other}").into()),
	}

	Ok(())
}

#[tokio::test]
async fn test_web_stream_sse_frame_tap_taps_all_blocks() -> Result<()> {
	// -- Setup & Fixtures
	// Includes a comment-only block and an event-without-data block: both are invisible to
	// the event stream, but a sink must still see them.
	let body = concat!(
		"event: message_start\n",
		"data: {\"a\":1}\n\n",
		": keep-alive\n\n",
		"event: ping\n\n",
		"data: {\"b\":2}\n\n",
		"data: [DONE]\n\n",
	);
	let raw_response = support_raw_ok_response("text/event-stream", body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let (sink, frame_tap) = support_new_frame_tap();

	let reqwest_builder = reqwest::Client::new().post(&url);
	let mut event_source = EventSourceStream::new(reqwest_builder).with_frame_tap(Some(frame_tap));

	// -- Exec
	let mut messages: Vec<Message> = Vec::new();
	while let Some(event) = event_source.next().await {
		if let Event::Message(message) = event.map_err(|err| err.to_string())? {
			messages.push(message);
		}
	}

	// -- Check
	// Only the three data-carrying blocks become events ..
	assert_eq!(messages.len(), 3, "should have 3 sse messages");
	// .. while the sink sees all five wire blocks.
	let frames = sink.frames();
	assert_eq!(frames.len(), 5, "sink should see every wire block");
	assert_eq!(
		frames.iter().map(|f| f.index).collect::<Vec<_>>(),
		vec![0, 1, 2, 3, 4],
		"frame indices should be contiguous from 0"
	);
	assert_eq!(frames[0].event.as_deref(), Some("message_start"));
	assert_eq!(
		frames[0].data.as_json().and_then(|v| v.get("a")).and_then(|v| v.as_i64()),
		Some(1)
	);
	assert_eq!(frames[1].event, None, "comment-only block has no event name");
	assert_eq!(frames[1].data.as_text(), Some(""), "comment-only block has no data");
	assert_eq!(
		frames[2].event.as_deref(),
		Some("ping"),
		"data-less event should be tapped"
	);
	assert_eq!(
		frames[4].data.as_text(),
		Some("[DONE]"),
		"non-json frame should stay text"
	);

	Ok(())
}

#[tokio::test]
async fn test_web_stream_delimited_frame_tap_taps_each_line() -> Result<()> {
	// -- Setup & Fixtures
	let body = "{\"a\":1}\n{\"b\":2}\nnot-json\n";
	let raw_response = support_raw_ok_response("application/x-ndjson", body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let (sink, frame_tap) = support_new_frame_tap();

	let reqwest_builder = reqwest::Client::new().post(&url);
	let mut web_stream = WebStream::new_with_delimiter(reqwest_builder, "\n").with_frame_tap(Some(frame_tap));

	// -- Exec
	let mut messages: Vec<String> = Vec::new();
	while let Some(message) = web_stream.next().await {
		messages.push(message.map_err(|err| err.to_string())?);
	}

	// -- Check
	assert_eq!(messages.len(), 3, "should have 3 ndjson messages");
	let frames = sink.frames();
	assert_eq!(frames.len(), 3, "sink should see every line");
	assert_eq!(frames.iter().map(|f| f.index).collect::<Vec<_>>(), vec![0, 1, 2]);
	assert!(
		frames.iter().all(|f| f.event.is_none()),
		"delimited transports have no event name"
	);
	assert_eq!(
		frames[1].data.as_json().and_then(|v| v.get("b")).and_then(|v| v.as_i64()),
		Some(2)
	);
	assert_eq!(
		frames[2].data.as_text(),
		Some("not-json"),
		"non-json line should stay text"
	);

	Ok(())
}

#[tokio::test]
async fn test_web_stream_no_sink_leaves_transport_untapped() -> Result<()> {
	// -- Setup & Fixtures
	let body = "{\"a\":1}\n";
	let raw_response = support_raw_ok_response("application/x-ndjson", body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;

	let reqwest_builder = reqwest::Client::new().post(&url);
	let mut web_stream = WebStream::new_with_delimiter(reqwest_builder, "\n").with_frame_tap(None);

	// -- Exec
	let mut messages: Vec<String> = Vec::new();
	while let Some(message) = web_stream.next().await {
		messages.push(message.map_err(|err| err.to_string())?);
	}

	// -- Check
	assert_eq!(messages.len(), 1);
	assert!(web_stream.frame_tap().is_none(), "no sink should mean no tap");

	Ok(())
}

// region:    --- Support

/// Spawns a one-shot HTTP server that answers the first request with the given raw HTTP response.
async fn support_spawn_one_shot_http_server(raw_response: String) -> Result<String> {
	let listener = TcpListener::bind("127.0.0.1:0").await?;
	let addr = listener.local_addr()?;
	tokio::spawn(async move {
		if let Ok((mut socket, _)) = listener.accept().await {
			// Best effort: read the (small) request bytes before responding.
			let mut buf = [0u8; 4096];
			let _ = socket.read(&mut buf).await;
			let _ = socket.write_all(raw_response.as_bytes()).await;
			let _ = socket.shutdown().await;
		}
	});
	Ok(format!("http://{addr}/"))
}

/// Builds a `CollectorSink` and the `FrameTap` that feeds it.
fn support_new_frame_tap() -> (Arc<CollectorSink>, FrameTap) {
	let sink = Arc::new(CollectorSink::new());
	let sink_dyn: Arc<dyn ChatFrameSink> = sink.clone();
	let ctx = FrameCtx::new(ModelIden::new(AdapterKind::OpenAI, "test-model"));

	(sink, FrameTap::new(sink_dyn, ctx))
}

/// Formats a 200 response with the given content type and body.
fn support_raw_ok_response(content_type: &str, body: &str) -> String {
	format!(
		"HTTP/1.1 200 OK\r\n\
		content-type: {content_type}\r\n\
		content-length: {}\r\n\
		connection: close\r\n\
		\r\n\
		{body}",
		body.len()
	)
}

// endregion: --- Support
