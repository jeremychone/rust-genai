//! API DOC: https://docs.anthropic.com/en/api/messages

use crate::utils::x_value::XValue;
use crate::{Error, Result};
pub use eventsource_stream::Event as MessageEvent;
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
	type Item = Result<AnthropicStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			// NOTE: At this point we capture more events than needed for genai::StreamItem, but it serves as documentation.
			//       see: https://docs.anthropic.com/en/api/messages
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(AnthropicStreamEvent::ConnectionOpen))),
				Some(Ok(Event::Message(message))) => match message.event.as_str() {
					"message_start" => return Poll::Ready(Some(Ok(AnthropicStreamEvent::MessageStart))),
					"message_delta" => {
						return Poll::Ready(Some(Ok(AnthropicStreamEvent::MessageDelta(message))));
					}
					"content_block_start" => return Poll::Ready(Some(Ok(AnthropicStreamEvent::BlockStart))),
					"content_block_delta" => {
						let mut data: Value = serde_json::from_str(&message.data).map_err(Error::StreamParse)?;
						let text: String = data.x_take("/delta/text")?;
						return Poll::Ready(Some(Ok(AnthropicStreamEvent::BlockDelta(text))));
					}
					"message_stop" => {
						// make sure we do not poll the EventSource anymore on next poll
						// NOTE: This way, the last MessageStop event is still sent,
						//       but then, nothing else will be processed
						self.done = true;
						return Poll::Ready(Some(Ok(AnthropicStreamEvent::MessageStop)));
					}
					"content_block_stop" => return Poll::Ready(Some(Ok(AnthropicStreamEvent::BlockStop))),
					"ping" => (), // loop to the next event
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

pub enum AnthropicStreamEvent {
	ConnectionOpen,
	MessageStart,
	MessageDelta(MessageEvent),
	BlockStart,
	BlockDelta(String),
	BlockStop,
	MessageStop,
}
