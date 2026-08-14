use super::*;
use crate::error::Error as GenaiError;
use futures::StreamExt;
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

// endregion: --- Support
