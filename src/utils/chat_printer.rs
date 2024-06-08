use crate::chat::{ChatStreamResponse, StreamChunk, ChatStreamEvent};
use futures::StreamExt;
use tokio::io::AsyncWriteExt as _;

/// Convenient function that print a chat stream and also capture the content and returns it.
pub async fn print_chat_stream(chat_res: ChatStreamResponse) -> Result<String, Box<dyn std::error::Error>> {
	let mut stdout = tokio::io::stdout();

	let mut stream = chat_res.stream;

	let mut content_capture = String::new();

	while let Some(Ok(stream_event)) = stream.next().await {
		let ChatStreamEvent::Chunk(StreamChunk { content }) = stream_event else {
			continue;
		};

		// Capture
		content_capture.push_str(&content);

		// Write output
		stdout.write_all(content.as_bytes()).await?;
		stdout.flush().await?;
	}

	stdout.write_all(b"\n").await?;
	stdout.flush().await?;

	Ok(content_capture)
}
