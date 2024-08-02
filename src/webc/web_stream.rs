use bytes::Bytes;
use futures::stream::TryStreamExt;
use futures::{Future, Stream};
use reqwest::{RequestBuilder, Response};
use std::collections::VecDeque;
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

/// WebStream is a simple web stream implementation that splits the stream message by a given delimiter.
/// - It is intended to be a pragmatic solution for services that do not adhere to the `text/event-stream` format and content-type.
/// - For providers that support the standard `text/event-stream`, `genai` uses the `reqwest-eventsource`/`eventsource-stream` crates.
/// - This stream item is just a `String` and has different stream modes that define the message delimiter strategy (without any event typing).
/// - Each "Event" is just a string-based and has only one event type, which is a string.
/// - It is the responsibility of the user of this stream to wrap it into a semantically correct stream of events depending on the domain.
#[allow(clippy::type_complexity)]
pub struct WebStream {
	stream_mode: StreamMode,
	reqwest_builder: Option<RequestBuilder>,
	response_future: Option<Pin<Box<dyn Future<Output = Result<Response, Box<dyn Error>>>>>>,
	bytes_stream: Option<Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn Error>>>>>>,
	// If a poll was a partial message, so we kept the previous part
	partial_message: Option<String>,
	// If a poll retrieved multiple messages, we keep to be sent in next poll
	remaining_messages: Option<VecDeque<String>>,
}

pub enum StreamMode {
	// This is use for Cohere with single `\n`
	Delimiter(&'static str),
	// This is for Gemini (standard json array, pretty formatted)
	PrettyJsonArray,
}

impl WebStream {
	pub fn new_with_delimiter(reqwest_builder: RequestBuilder, message_delimiter: &'static str) -> Self {
		Self {
			stream_mode: StreamMode::Delimiter(message_delimiter),
			reqwest_builder: Some(reqwest_builder),
			response_future: None,
			bytes_stream: None,
			partial_message: None,
			remaining_messages: None,
		}
	}

	pub fn new_with_pretty_json_array(reqwest_builder: RequestBuilder) -> Self {
		Self {
			stream_mode: StreamMode::PrettyJsonArray,
			reqwest_builder: Some(reqwest_builder),
			response_future: None,
			bytes_stream: None,
			partial_message: None,
			remaining_messages: None,
		}
	}
}

impl Stream for WebStream {
	type Item = Result<String, Box<dyn Error>>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// -- First, we check if we have any remaining messages to send.
		if let Some(ref mut remaining_messages) = this.remaining_messages {
			if let Some(msg) = remaining_messages.pop_front() {
				return Poll::Ready(Some(Ok(msg)));
			}
		}

		// -- Then execute the web poll and processing loop
		loop {
			if let Some(ref mut fut) = this.response_future {
				match Pin::new(fut).poll(cx) {
					Poll::Ready(Ok(response)) => {
						let bytes_stream = response.bytes_stream().map_err(|e| Box::new(e) as Box<dyn Error>);
						this.bytes_stream = Some(Box::pin(bytes_stream));
						this.response_future = None;
					}
					Poll::Ready(Err(e)) => {
						this.response_future = None;
						return Poll::Ready(Some(Err(e)));
					}
					Poll::Pending => return Poll::Pending,
				}
			}

			if let Some(ref mut stream) = this.bytes_stream {
				match stream.as_mut().poll_next(cx) {
					Poll::Ready(Some(Ok(bytes))) => {
						let buff_string = match String::from_utf8(bytes.to_vec()) {
							Ok(s) => s,
							Err(e) => return Poll::Ready(Some(Err(Box::new(e) as Box<dyn Error>))),
						};

						//  -- iterate through the parts
						let buff_response = match this.stream_mode {
							StreamMode::Delimiter(delimiter) => {
								process_buff_string_delimited(buff_string, &mut this.partial_message, delimiter)
							}
							StreamMode::PrettyJsonArray => {
								new_with_pretty_json_array(buff_string, &mut this.partial_message)
							}
						};

						let BuffResponse {
							mut first_message,
							next_messages,
							candidate_message,
						} = buff_response?;

						// -- Add next_messages as remaining messages if present
						if let Some(next_messages) = next_messages {
							this.remaining_messages.get_or_insert(VecDeque::new()).extend(next_messages);
						}

						// -- If still have a candidate, it's the partial for next one
						if let Some(candidate_message) = candidate_message {
							// for now, we just log this
							if this.partial_message.is_some() {
								println!("GENAI - WARNING - partial_message is not none");
							}
							this.partial_message = Some(candidate_message);
						}

						// -- If we have a first message, we have to send it.
						if let Some(first_message) = first_message.take() {
							return Poll::Ready(Some(Ok(first_message)));
						} else {
							continue;
						}
					}
					Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
					Poll::Ready(None) => {
						if let Some(partial) = this.partial_message.take() {
							if !partial.is_empty() {
								return Poll::Ready(Some(Ok(partial)));
							}
						}
						this.bytes_stream = None;
					}
					Poll::Pending => return Poll::Pending,
				}
			}

			if let Some(reqwest_builder) = this.reqwest_builder.take() {
				let fut = async move { reqwest_builder.send().await.map_err(|e| Box::new(e) as Box<dyn Error>) };
				this.response_future = Some(Box::pin(fut));
				continue;
			}

			return Poll::Ready(None);
		}
	}
}

struct BuffResponse {
	first_message: Option<String>,
	next_messages: Option<Vec<String>>,
	candidate_message: Option<String>,
}

/// Process a string buffer for the pretty_json_array (for Gemini)
/// It will split the messages as such:
/// - if start with `[` then, the message will be `[`
/// - Then, each json main object (from first `{` to last `}`) will become a message
/// - Main json object `,` delimiter will be skipped
/// - ending `]` will be sent as a `]` message as well.
///
/// IMPORTANT: Right now, it assumes each buff_string will contain the full main json object
///            for each array item (Which seems to be the case with Gemini)
///            This probably need to be made more robust later
fn new_with_pretty_json_array(
	buff_string: String,
	_partial_message: &mut Option<String>,
) -> Result<BuffResponse, crate::Error> {
	let buff_str = buff_string.trim();

	let mut messages: Vec<String> = Vec::new();

	// -- Capture the array start/end and each eventual sub object (assuming only one sub object)
	let (array_start, rest_str) = match buff_str.strip_prefix('[') {
		Some(rest) => (Some("["), rest.trim()),
		None => (None, buff_str),
	};

	// remove the eventual ',' prefix and suffix.
	let rest_str = rest_str.strip_prefix(',').unwrap_or(rest_str);
	let rest_str = rest_str.strip_suffix(',').unwrap_or(rest_str);

	let (rest_str, array_end) = match rest_str.strip_suffix(']') {
		Some(rest) => (rest.trim(), Some("]")),
		None => (rest_str, None),
	};

	// -- Prep the BuffResponse
	if let Some(array_start) = array_start {
		messages.push(array_start.to_string());
	}
	if !rest_str.is_empty() {
		messages.push(rest_str.to_string());
	}
	// we ignore the comma
	if let Some(array_end) = array_end {
		messages.push(array_end.to_string());
	}

	// -- Return the buf response
	let first_message = if !messages.is_empty() {
		Some(messages[0].to_string())
	} else {
		None
	};

	let next_messages = if messages.len() > 1 {
		Some(messages[1..].to_vec())
	} else {
		None
	};

	Ok(BuffResponse {
		first_message,
		next_messages,
		candidate_message: None,
	})
}

/// Process a string buffer for the delimited mode (e.g., Cohere)
fn process_buff_string_delimited(
	buff_string: String,
	partial_message: &mut Option<String>,
	delimiter: &str,
) -> Result<BuffResponse, crate::Error> {
	let mut first_message: Option<String> = None;
	let mut candidate_message: Option<String> = None;
	let mut next_messages: Option<Vec<String>> = None;

	let parts = buff_string.split(delimiter);

	for part in parts {
		// if we already have a candidate, the candidate become the message
		if let Some(candidate_message) = candidate_message.take() {
			// if candidate is empty, we skip
			if !candidate_message.is_empty() {
				let message = candidate_message.to_string();
				if first_message.is_none() {
					first_message = Some(message);
				} else {
					next_messages.get_or_insert_with(Vec::new).push(message);
				}
			} else {
				continue;
			}
		} else {
			// and then, this part becomes the candidate
			if let Some(partial) = partial_message.take() {
				candidate_message = Some(format!("{partial}{part}"));
			} else {
				candidate_message = Some(part.to_string());
			}
		}
	}

	Ok(BuffResponse {
		first_message,
		next_messages,
		candidate_message,
	})
}
