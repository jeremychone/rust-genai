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
	let mut web_stream = WebStream::new_with_sse(RequestBuilder::new(reqwest_builder, None, None));

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
	let mut event_source =
		EventSourceStream::new(RequestBuilder::new(reqwest_builder, None, None)).with_frame_tap(Some(frame_tap));

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
async fn test_event_source_stream_json_body_is_not_sse() -> Result<()> {
	// -- Setup & Fixtures
	// A gateway that ignores `stream: true` and answers one completion.
	let body = r#"{"id":"c1","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"Hello"},"finish_reason":"stop"}]}"#;
	let raw_response = support_raw_ok_response("application/json", body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let reqwest_builder = reqwest::Client::new().post(&url);
	let mut event_source = EventSourceStream::new(RequestBuilder::new(reqwest_builder, None, None));

	// -- Exec
	let items: Vec<_> = (&mut event_source).collect().await;

	// -- Check
	assert_eq!(items.len(), 2, "should be Open, then the error");
	assert!(matches!(items[0], Ok(Event::Open)));
	let err = items
		.into_iter()
		.nth(1)
		.and_then(|item| item.err())
		.ok_or("Second item should be an error")?;
	let err = err
		.downcast::<GenaiError>()
		.map_err(|err| format!("Error should downcast to genai Error, but was: {err}"))?;
	match *err {
		GenaiError::StreamResponseNotSse {
			status,
			content_type,
			body: err_body,
		} => {
			assert_eq!(status.as_u16(), 200);
			assert_eq!(content_type, "application/json");
			assert_eq!(err_body, body);
		}
		other => return Err(format!("Should be an Error::StreamResponseNotSse, but was: {other}").into()),
	}
	assert!(event_source.next().await.is_none(), "stream should end after the error");

	Ok(())
}

#[tokio::test]
async fn test_event_source_stream_not_sse_body_is_capped() -> Result<()> {
	// -- Setup & Fixtures
	// A web page with a blank line, so it arrives as two blocks, kept as `<html>\n\n` (8 bytes)
	// and then the second block, where the 2-byte `é` spans bytes 4095-4096 and must be cut whole.
	let body = format!("<html>\n\n<p>{}é</p>\n\n<p>more</p></html>\n", "a".repeat(4084));
	let raw_response = support_raw_ok_response("text/html; charset=utf-8", &body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let reqwest_builder = reqwest::Client::new().post(&url);
	let event_source = EventSourceStream::new(RequestBuilder::new(reqwest_builder, None, None));

	// -- Exec
	let mut errors: Vec<BoxError> = event_source.filter_map(|item| async { item.err() }).collect().await;

	// -- Check
	assert_eq!(errors.len(), 1, "should end with one error");
	let err = errors
		.pop()
		.ok_or("Should have an error")?
		.downcast::<GenaiError>()
		.map_err(|err| format!("Error should downcast to genai Error, but was: {err}"))?;
	match *err {
		GenaiError::StreamResponseNotSse {
			content_type,
			body: err_body,
			..
		} => {
			assert_eq!(content_type, "text/html; charset=utf-8");
			assert!(err_body.starts_with("<html>\n\n<p>aaa"), "body: {err_body}");
			assert_eq!(err_body.len(), 4095, "the cut should fall before the `é`");
			assert!(err_body.ends_with('a'));
		}
		other => return Err(format!("Should be an Error::StreamResponseNotSse, but was: {other}").into()),
	}

	Ok(())
}

#[tokio::test]
async fn test_event_source_stream_comment_only_body_ends_quietly() -> Result<()> {
	// -- Setup & Fixtures
	// Valid SSE that carries no event: not a format problem, so no error.
	let body = ": keep-alive\n\nid: 1\nretry: 1000\n\nevent\n\n";
	let raw_response = support_raw_ok_response("text/event-stream", body);
	let url = support_spawn_one_shot_http_server(raw_response).await?;
	let reqwest_builder = reqwest::Client::new().post(&url);
	let event_source = EventSourceStream::new(RequestBuilder::new(reqwest_builder, None, None));

	// -- Exec
	let items: Vec<_> = event_source.collect().await;

	// -- Check
	assert_eq!(items.len(), 1, "should only be Open, was: {items:?}");
	assert!(matches!(items[0], Ok(Event::Open)));

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
	let mut web_stream = WebStream::new_with_delimiter(RequestBuilder::new(reqwest_builder, None, None), "\n")
		.with_frame_tap(Some(frame_tap));

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
	let mut web_stream =
		WebStream::new_with_delimiter(RequestBuilder::new(reqwest_builder, None, None), "\n").with_frame_tap(None);

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

#[tokio::test]
async fn test_web_stream_utf8_split_character() -> Result<()> {
	// -- Setup & Fixtures
	let reqwest_builder = reqwest::Client::new().get("http://127.0.0.1/");
	let mut web_stream = WebStream::new_with_delimiter(RequestBuilder::new(reqwest_builder, None, None), "\n");
	web_stream.reqwest_builder = None;
	web_stream.bytes_stream = Some(Box::pin(futures::stream::iter([
		Ok::<_, BoxError>(Bytes::from_static(b"caf\xc3")),
		Ok::<_, BoxError>(Bytes::from_static(b"\xa9\n")),
	])));

	// -- Exec
	let message = web_stream.next().await.ok_or("Should have a decoded message")?;
	let message = message.map_err(|err| err.to_string())?;
	let end = web_stream.next().await;

	// -- Check
	assert_eq!(message, "caf\u{e9}");
	assert!(end.is_none());

	Ok(())
}

#[tokio::test]
async fn test_web_stream_utf8_invalid_bytes() -> Result<()> {
	// -- Setup & Fixtures
	let reqwest_builder = reqwest::Client::new().get("http://127.0.0.1/");
	let mut web_stream = WebStream::new_with_delimiter(RequestBuilder::new(reqwest_builder, None, None), "\n");
	web_stream.reqwest_builder = None;
	web_stream.bytes_stream = Some(Box::pin(futures::stream::iter([
		Ok::<_, BoxError>(Bytes::from_static(b"caf\xc3")),
		Ok::<_, BoxError>(Bytes::from_static(b"\xff\n")),
		Ok::<_, BoxError>(Bytes::from_static(b"more\n")),
	])));

	// -- Exec
	let item = web_stream.next().await.ok_or("Should have an error item")?;
	let end = web_stream.next().await;

	// -- Check
	let error = item.err().ok_or("Invalid UTF-8 should return an error")?;
	let error = error
		.downcast_ref::<std::string::FromUtf8Error>()
		.ok_or("Invalid UTF-8 should retain the original bytes in FromUtf8Error")?;
	assert_eq!(error.as_bytes(), b"caf\xc3\xff\n");
	assert_eq!(error.utf8_error().valid_up_to(), 3);
	assert_eq!(error.utf8_error().error_len(), Some(1));
	assert!(end.is_none(), "invalid UTF-8 should end the stream");

	Ok(())
}

#[tokio::test]
async fn test_web_stream_utf8_truncated_at_end() -> Result<()> {
	// -- Setup & Fixtures
	let reqwest_builder = reqwest::Client::new().get("http://127.0.0.1/");
	let mut web_stream = WebStream::new_with_delimiter(RequestBuilder::new(reqwest_builder, None, None), "\n");
	web_stream.reqwest_builder = None;
	web_stream.bytes_stream = Some(Box::pin(futures::stream::iter([Ok::<_, BoxError>(
		Bytes::from_static(b"ok\ncaf\xc3"),
	)])));

	// -- Exec
	let first = web_stream.next().await.ok_or("Should have a first message")?;
	let first = first.map_err(|err| err.to_string())?;
	let item = web_stream.next().await.ok_or("Should have an error item")?;
	let end = web_stream.next().await;

	// -- Check
	assert_eq!(first, "ok");
	let error = item.err().ok_or("Truncated UTF-8 at end should return an error")?;
	let error = error
		.downcast_ref::<std::string::FromUtf8Error>()
		.ok_or("Truncated UTF-8 should retain the undelivered bytes in FromUtf8Error")?;
	assert_eq!(error.as_bytes(), b"caf\xc3");
	assert_eq!(error.utf8_error().valid_up_to(), 3);
	assert_eq!(error.utf8_error().error_len(), None);
	assert!(end.is_none(), "stream should end after the truncation error");

	Ok(())
}

#[tokio::test]
async fn test_web_stream_sse_crlf_split_across_chunks() -> Result<()> {
	// -- Setup & Fixtures
	// One split inside the event (after `event:`), one inside the blank line ending it.
	let chunks: &[&[u8]] = &[
		b"event: content_block_delta\r",
		b"\ndata: {\"a\":1}\r\n\r",
		b"\nevent: message_stop\r\n",
		b"data: {\"b\":2}\r\n\r\n",
	];

	// -- Exec
	let messages = support_collect_sse_messages(chunks).await?;

	// -- Check
	assert_eq!(
		messages,
		vec![
			"event: content_block_delta\ndata: {\"a\":1}",
			"event: message_stop\ndata: {\"b\":2}",
		]
	);

	Ok(())
}

#[tokio::test]
async fn test_web_stream_sse_crlf_split_by_empty_chunk() -> Result<()> {
	// -- Setup & Fixtures
	let chunks: &[&[u8]] = &[b"event: a\r", b"", b"\ndata: 1\r\n\r\n"];

	// -- Exec
	let messages = support_collect_sse_messages(chunks).await?;

	// -- Check
	assert_eq!(messages, vec!["event: a\ndata: 1"]);

	Ok(())
}

#[tokio::test]
async fn test_web_stream_sse_cr_line_ends_across_chunks() -> Result<()> {
	// -- Setup & Fixtures
	// A lone `\r` ending a chunk is a line end on its own: the next `\r\n` still makes a blank line.
	let chunks: &[&[u8]] = &[b"data: 1\r", b"\r\n", b"event: b\r", b"data: 2\r\r"];

	// -- Exec
	let messages = support_collect_sse_messages(chunks).await?;

	// -- Check
	assert_eq!(messages, vec!["data: 1", "event: b\ndata: 2"]);

	Ok(())
}

// region:    --- Support

/// Feeds the given chunks to an SSE `WebStream` and collects every message it yields.
async fn support_collect_sse_messages(chunks: &[&'static [u8]]) -> Result<Vec<String>> {
	let reqwest_builder = reqwest::Client::new().get("http://127.0.0.1/");
	let mut web_stream = WebStream::new_with_sse(RequestBuilder::new(reqwest_builder, None, None));
	web_stream.reqwest_builder = None;
	let chunks: Vec<_> = chunks.iter().map(|c| Ok::<_, BoxError>(Bytes::from_static(c))).collect();
	web_stream.bytes_stream = Some(Box::pin(futures::stream::iter(chunks)));

	let mut messages = Vec::new();
	while let Some(message) = web_stream.next().await {
		messages.push(message.map_err(|err| err.to_string())?);
	}
	Ok(messages)
}

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
