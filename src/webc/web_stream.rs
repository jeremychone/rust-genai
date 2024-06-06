use bytes::Bytes;
use futures::stream::TryStreamExt;
use futures::{Future, Stream};
use reqwest::{RequestBuilder, Response};
use std::collections::VecDeque;
use std::error::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

/// WebStream is a simple web stream implementation that splits the stream message by a given delimiter.
/// - It's intended to be a pragmatic solution for services that do not adhere to the `text/event-stream` format and content-type.
/// - For providers that support the standard `text/event-stream`, `genai` uses the `reqwest-eventsource`/`eventsource-stream` crates.
#[allow(clippy::type_complexity)]
pub struct WebStream {
	message_delimiter: &'static str,
	reqwest_builder: Option<RequestBuilder>,
	response_future: Option<Pin<Box<dyn Future<Output = Result<Response, Box<dyn Error>>>>>>,
	bytes_stream: Option<Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn Error>>>>>>,
	// If a poll was a partial message, so we kept the previous part
	partial_message: Option<String>,
	// If a poll retrieved multiple messages, we keep to be sent in next poll
	remaining_messages: Option<VecDeque<String>>,
}

impl WebStream {
	pub fn new(reqwest_builder: RequestBuilder, message_delimiter: &'static str) -> Self {
		Self {
			message_delimiter,
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
						let string = match String::from_utf8(bytes.to_vec()) {
							Ok(s) => s,
							Err(e) => return Poll::Ready(Some(Err(Box::new(e) as Box<dyn Error>))),
						};

						//  -- iterate through the parts
						let parts = string.split(this.message_delimiter);
						let mut first_message: Option<String> = None;
						let mut candidate_message: Option<String> = None;

						for part in parts {
							// if we already have a candidate, the candidate become the message
							if let Some(canditate_message) = candidate_message.take() {
								// if canditate is empty, we skip
								if !canditate_message.is_empty() {
									let message = canditate_message.to_string();
									if first_message.is_none() {
										first_message = Some(message);
									} else {
										this.remaining_messages.get_or_insert_with(VecDeque::new).push_back(message);
									}
								} else {
									continue;
								}
							} else {
								// and then, this part becomes the candidate
								if let Some(partial) = this.partial_message.take() {
									candidate_message = Some(format!("{partial}{part}"));
								} else {
									candidate_message = Some(part.to_string());
								}
							}
						}

						// -- if se still have a candidate, it's the partial for next one
						if let Some(candidate_message) = candidate_message {
							if this.partial_message.is_some() {
								println!("GENAI - WARNING - partial_message should be none (ignoring)");
							}
							this.partial_message = Some(candidate_message);
						}

						// -- if we have a furst message, we nave to send it.
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
