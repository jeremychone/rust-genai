use crate::utils::x_value::XValue;
use crate::webc::WebStream;
use crate::{Error, Result};
pub use eventsource_stream::Event as MessageEvent;
use reqwest_eventsource::{Event, EventSource};
use serde::Deserialize;
use serde_json::Value;
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

impl futures::Stream for CohereStream {
	type Item = Result<CohereStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		match Pin::new(&mut self.inner).poll_next(cx) {
			Poll::Ready(Some(Ok(raw_string))) => {
				let cohere_message =
					serde_json::from_str::<CohereStreamMessage>(&raw_string).map_err(Error::StreamParse);

				match cohere_message {
					Ok(cohere_message) => match cohere_message.event_type.as_str() {
						"stream-start" => Poll::Ready(Some(Ok(CohereStreamEvent::StreamStart))),
						"text-generation" => Poll::Ready(Some(Ok(CohereStreamEvent::Chunk(cohere_message.text)))),
						// TODO: need capture the data on stream-end (meta and such at least)
						"stream-end" => Poll::Ready(Some(Ok(CohereStreamEvent::StreamEnd))),
						_ => Poll::Ready(Some(Ok(CohereStreamEvent::Other))),
					},
					Err(err) => {
						println!("Cohere Adapter Stream Error: {}", err);
						Poll::Ready(Some(Err(err)))
					}
				}
			}
			Poll::Ready(Some(Err(err))) => {
				println!("Cohere Adapter Stream Error: {}", err);
				Poll::Ready(Some(Err(Error::WebStream)))
			}
			Poll::Ready(None) => {
				self.done = true; // not needed for now
				Poll::Ready(None)
			}
			Poll::Pending => Poll::Pending,
		}
	}
}

pub enum CohereStreamEvent {
	StreamStart,
	Chunk(Option<String>),
	StreamEnd,
	Other,
}

#[derive(Deserialize, Debug)]
struct CohereStreamMessage {
	is_finished: bool,
	event_type: String,
	text: Option<String>,
}
