use crate::chat::ChatStream;
use futures::StreamExt;
use tokio::io::AsyncWriteExt as _;

/// Convenient function that print a chat stream and also capture the content and returns it.
pub async fn print_chat_stream(chat_res: ChatStream) -> Result<String, Box<dyn std::error::Error>> {
	let mut stdout = tokio::io::stdout();

	let mut stream = chat_res.stream;

	let mut content_capture = String::new();

	while let Some(Ok(stream_item)) = stream.next().await {
		let Some(content) = stream_item.content else {
			// TODO: For debug only right now (should not have empty content stream_item)
			stdout.write_all(b"\nEMPTY RESPONSE - CONTINUE\n").await?;
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
