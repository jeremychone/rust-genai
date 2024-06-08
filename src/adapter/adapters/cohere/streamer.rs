use crate::adapter::inter_stream::InterStreamEvent;
use crate::webc::WebStream;
use crate::{Error, Result};
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct CohereStream {
	inner: WebStream,
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
}

impl CohereStream {
	pub fn new(inner: WebStream) -> Self {
		CohereStream { inner, done: false }
	}
}

#[derive(Deserialize, Debug)]
struct CohereStreamMessage {
	#[allow(unused)]
	is_finished: bool,
	event_type: String,
	text: Option<String>,
}

// Implement futures::Stream for InterStream<CohereStream>
impl futures::Stream for CohereStream {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(raw_string)) => {
					let cohere_message =
						serde_json::from_str::<CohereStreamMessage>(&raw_string).map_err(Error::StreamParse);

					match cohere_message {
						Ok(cohere_message) => {
							let inter_event = match cohere_message.event_type.as_str() {
								"stream-start" => InterStreamEvent::Start,
								"text-generation" => InterStreamEvent::Chunk(cohere_message.text.unwrap_or_default()),
								"stream-end" => InterStreamEvent::End,
								_ => continue, // Skip the "Other" event
							};
							return Poll::Ready(Some(Ok(inter_event)));
						}
						Err(err) => {
							println!("Cohere Adapter Stream Error: {}", err);
							return Poll::Ready(Some(Err(err)));
						}
					}
				}
				Some(Err(err)) => {
					println!("Cohere Adapter Stream Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream)));
				}
				None => {
					self.done = true;
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}
