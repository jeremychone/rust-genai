use crate::adapter::inter_stream::InterStreamEvent;
use crate::utils::x_value::XValue;
use crate::{Error, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct AnthropicMessagesStream {
	inner: EventSource,
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
}

impl AnthropicMessagesStream {
	pub fn new(inner: EventSource) -> Self {
		AnthropicMessagesStream { inner, done: false }
	}
}

impl futures::Stream for AnthropicMessagesStream {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			// NOTE: At this point we capture more events than needed for genai::StreamItem, but it serves as documentation.
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => match message.event.as_str() {
					"message_start" => continue,
					"message_delta" => continue,
					"content_block_start" => continue,
					"content_block_delta" => {
						let mut data: Value = serde_json::from_str(&message.data).map_err(Error::StreamParse)?;
						let text: String = data.x_take("/delta/text")?;
						return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(text))));
					}
					"content_block_stop" => continue,
					"message_stop" => {
						// make sure we do not poll the EventSource anymore on next poll
						// NOTE: This way, the last MessageStop event is still sent,
						//       but then, nothing else will be processed
						self.done = true;
						return Poll::Ready(Some(Ok(InterStreamEvent::End)));
					}

					"ping" => continue, // loop to the next event
					other => println!("UNKNOWN MESSAGE TYPE: {other}"),
				},
				Some(Err(err)) => {
					println!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::ReqwestEventSource(err))));
				}
				None => return Poll::Ready(None),
			}
		}
		Poll::Pending
	}
}
