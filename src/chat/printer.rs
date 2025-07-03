//! Printer utility to help print a chat stream
//! > Note: This is primarily for quick testing and temporary debugging

use crate::chat::{ChatStreamEvent, ChatStreamResponse, StreamChunk};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWriteExt as _, Stdout};

// Note: This module has its own Error type (see end of file)
type Result<T> = core::result::Result<T, Error>;

// region:    --- PrintChatOptions

/// Options to be passed into the `printer::print_chat_stream`
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PrintChatStreamOptions {
	print_events: Option<bool>,
}

/// Constructors
impl PrintChatStreamOptions {
	/// Create a `PrintChatStreamOptions` with the `print_events` field set to `true`
	pub fn from_print_events(print_events: bool) -> Self {
		PrintChatStreamOptions {
			print_events: Some(print_events),
		}
	}
}

// endregion: --- PrintChatOptions

/// Convenient function that prints a chat stream, captures the content, and returns it (only concatenated chunks).
pub async fn print_chat_stream(
	chat_res: ChatStreamResponse,
	options: Option<&PrintChatStreamOptions>,
) -> Result<String> {
	let mut stdout = tokio::io::stdout();
	let res = print_chat_stream_inner(&mut stdout, chat_res, options).await;
	// Ensure tokio stdout flush is called, regardless of success or failure.
	stdout.flush().await?;
	res
}

async fn print_chat_stream_inner(
	stdout: &mut Stdout,
	chat_res: ChatStreamResponse,
	options: Option<&PrintChatStreamOptions>,
) -> Result<String> {
	let mut stream = chat_res.stream;

	let mut content_capture = String::new();

	let print_events = options.and_then(|o| o.print_events).unwrap_or_default();

	let mut first_chunk = true;
	let mut first_reasoning_chunk = true;
	let mut first_tool_chunk = true;

	while let Some(Ok(stream_event)) = stream.next().await {
		let (event_info, content) = {
			match stream_event {
				ChatStreamEvent::Start => {
					if print_events {
						// TODO: Might implement pretty JSON formatting
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

				ChatStreamEvent::ReasoningChunk(StreamChunk { content }) => {
					if print_events && first_reasoning_chunk {
						first_reasoning_chunk = false;
						(
							Some("\n-- ChatStreamEvent::ReasoningChunk (concatenated):\n".to_string()),
							Some(content),
						)
					} else {
						(None, Some(content))
					}
				}

				ChatStreamEvent::ToolCallChunk(tool_chunk) => {
					if print_events && first_tool_chunk {
						first_tool_chunk = false;
						(
							Some(format!(
								"\n-- ChatStreamEvent::ToolCallChunk: fn: {}, args: {}\n",
								tool_chunk.tool_call.fn_name, tool_chunk.tool_call.fn_arguments
							)),
							None,
						)
					} else {
						(None, None)
					}
				}

				ChatStreamEvent::End(end_event) => {
					if print_events {
						// TODO: Might implement pretty JSON formatting
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

// region:    --- Error

// Note 1: The printer has its own error type because it is more of a utility, and therefore
//         making the main crate error aware of the different error types would be unnecessary.
//
// Note 2: This Printer Error is not wrapped in the main crate error because the printer
//         functions are not used by any other crate functions (they are more of a debug utility)

use derive_more::From;

/// The Printer error.
#[derive(Debug, From)]
pub enum Error {
	/// The `tokio::io::Error` when using `tokio::io::stdout`
	#[from]
	TokioIo(tokio::io::Error),
}

// region:    --- Error Boilerplate

impl core::fmt::Display for Error {
	fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
		write!(fmt, "{self:?}")
	}
}

impl std::error::Error for Error {}

// endregion: --- Error Boilerplate
