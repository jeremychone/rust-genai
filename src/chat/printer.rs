use crate::chat::{ChatStreamEvent, ChatStreamResponse, StreamChunk};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWriteExt as _, Stdout};

// region:    --- PrintChatOptions

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PrintChatStreamOptions {
	print_events: Option<bool>,
}

/// constructors
impl PrintChatStreamOptions {
	pub fn from_print_events(print_events: bool) -> Self {
		PrintChatStreamOptions {
			print_events: Some(print_events),
		}
	}
}

// endregion: --- PrintChatOptions

/// Convenient function that print a chat stream and also capture the content and returns it (only concatenated chunks).
pub async fn print_chat_stream(
	chat_res: ChatStreamResponse,
	options: Option<&PrintChatStreamOptions>,
) -> Result<String, Box<dyn std::error::Error>> {
	let mut stdout = tokio::io::stdout();
	let res = print_chat_stream_inner(&mut stdout, chat_res, options).await;
	// make sure tokio stdout flush get called, regardless of success or not.
	stdout.flush().await?;
	res
}

async fn print_chat_stream_inner(
	stdout: &mut Stdout,
	chat_res: ChatStreamResponse,
	options: Option<&PrintChatStreamOptions>,
) -> Result<String, Box<dyn std::error::Error>> {
	let mut stream = chat_res.stream;

	let mut content_capture = String::new();

	let print_events = options.and_then(|o| o.print_events).unwrap_or_default();

	let mut first_chunk = true;

	while let Some(Ok(stream_event)) = stream.next().await {
		let (event_info, content) = {
			match stream_event {
				ChatStreamEvent::Start => {
					if print_events {
						// TODO: Might do a pretty json format
						(Some("\n-- ChatStreamEvent::Start\n".to_string()), None)
					} else {
						(None, None)
					}
				}

				ChatStreamEvent::Chunk(StreamChunk { content }) => {
					if print_events && first_chunk {
						first_chunk = false;
						(
							Some("\n-- ChatStreamEvent::Chunk (concatenated):\n".to_string()),
							Some(content),
						)
					} else {
						(None, Some(content))
					}
				}

				ChatStreamEvent::End(end_event) => {
					if print_events {
						// TODO: Might do a pretty json format
						(Some(format!("\n\n-- ChatStreamEvent::End {end_event:?}\n")), None)
					} else {
						(None, None)
					}
				}
			}
		};

		if let Some(event_info) = event_info {
			stdout.write_all(event_info.as_bytes()).await?;
		}

		if let Some(content) = content {
			content_capture.push_str(&content);
			stdout.write_all(content.as_bytes()).await?;
		};

		stdout.flush().await?;
	}

	stdout.write_all(b"\n").await?;

	Ok(content_capture)
}
