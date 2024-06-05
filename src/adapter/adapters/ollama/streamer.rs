//! API DOC: https://platform.openai.com/docs/api-reference/chat

use crate::utils::x_value::XValue;
use crate::{Error, Result};
pub use eventsource_stream::Event as MessageEvent;
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct OllamaStream {
	inner: EventSource,
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
}

impl OllamaStream {
	pub fn new(inner: EventSource) -> Self {
		OllamaStream { inner, done: false }
	}
}

impl futures::Stream for OllamaStream {
	type Item = Result<OllamaStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			println!("->> EVENT:\n{event:?}");
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(OllamaStreamEvent::Open))),
				Some(Ok(Event::Message(message))) => {
					if message.event == "message" {
						let mut message_data: Value =
							serde_json::from_str(&message.data).map_err(Error::StreamParse)?;

						let mut first_choice: Value = message_data.x_take("/choices/0")?;

						if let Some(finish_reason) = first_choice.x_take::<Option<String>>("finish_reason")? {
							self.done = true;
							return Poll::Ready(Some(Ok(OllamaStreamEvent::Finish(finish_reason))));
						} else if let Some(content) = first_choice.x_take::<Option<String>>("/delta/content")? {
							return Poll::Ready(Some(Ok(OllamaStreamEvent::Chunck(content))));
						} else {
							println!("EMPTY CHOICE CONTENT");
						}
					} else {
						println!("UNKNOWN message type '{}'", message.event);
					}
				}
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

pub enum OllamaStreamEvent {
	Open,
	Chunck(String),
	Finish(String),
}
