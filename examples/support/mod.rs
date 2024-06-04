//! Some support function of the examples

use futures::StreamExt;
use genai::ChatStream;
use tokio::io::AsyncWriteExt as _;

pub fn has_env(env_name: &str) -> bool {
	std::env::var(env_name).is_ok()
}

// Convenient function that print a chat stream and also capture the content and returns it.
pub async fn print_chat_stream(chat_res: ChatStream) -> Result<String, Box<dyn std::error::Error>> {
	let mut stdout = tokio::io::stdout();
	let mut char_count = 0;

	// let mut final_data_responses = Vec::new();
	let mut stream = chat_res.stream;

	let mut content_capture = String::new();

	while let Some(Ok(stream_item)) = stream.next().await {
		let Some(content) = stream_item.content else {
			stdout.write_all(b"\nEMPTY RESPONSE - CONTINUE\n").await?;
			continue;
		};

		// -- Add it to the capture
		content_capture.push_str(&content);

		// -- Print it and cheap text-wrapping
		let bytes = content.as_bytes();

		// Poor man's wrapping
		char_count += bytes.len();
		if char_count > 80 {
			stdout.write_all(b"\n").await?;
			char_count = 0;
		}

		// Write output
		stdout.write_all(bytes).await?;
		stdout.flush().await?;
	}

	stdout.write_all(b"\n").await?;
	stdout.flush().await?;

	Ok(content_capture)
}
