use crate::error::BoxError;
use crate::webc::{FrameTap, WebStream};
use futures::Stream;
use reqwest::RequestBuilder;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Simple EventSource stream implementation that uses WebStream as a foundation.
pub struct EventSourceStream {
	inner: WebStream,
	opened: bool,
	frame_tap: Option<FrameTap>,
}

#[derive(Debug)]
pub enum Event {
	Open,
	Message(Message),
}

#[derive(Debug)]
pub struct Message {
	pub event: String,
	pub data: String,
}

impl EventSourceStream {
	pub fn new(reqwest_builder: RequestBuilder) -> Self {
		// SSE event separator is `\n\n`, `\r\n\r\n`, or `\r\r`. WebStream's Sse mode
		// normalizes CR/CRLF to LF and splits on `\n\n`, so all three forms work.
		Self {
			inner: WebStream::new_with_sse(reqwest_builder),
			opened: false,
			frame_tap: None,
		}
	}

	/// Sets the frame tap that feeds the user `ChatFrameSink`.
	/// No-op path when `frame_tap` is `None` (i.e., no sink configured).
	pub fn with_frame_tap(mut self, frame_tap: Option<FrameTap>) -> Self {
		self.frame_tap = frame_tap;
		self
	}

	/// Clones the frame tap, for the terminal `on_end` / `on_error` hooks.
	pub fn frame_tap(&self) -> Option<FrameTap> {
		self.frame_tap.clone()
	}
}

impl Stream for EventSourceStream {
	type Item = Result<Event, BoxError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let this = self.get_mut();

		// -- 1. Handle initial "Open" event
		if !this.opened {
			this.opened = true;
			return Poll::Ready(Some(Ok(Event::Open)));
		}

		// -- 2. Poll the inner WebStream for next event block
		loop {
			let nx = Pin::new(&mut this.inner).poll_next(cx);

			match nx {
				Poll::Ready(Some(Ok(raw_event))) => {
					// `None` until an explicit `event:` line is seen; the SSE default name is
					// applied when building the `Message`, but the tap reports what was on the wire.
					let mut event: Option<String> = None;
					let mut data = String::new();
					for line in raw_event.lines() {
						let line = line.trim();
						// Skip empty lines or comments (starting with :)
						if line.is_empty() || line.starts_with(':') {
							continue;
						}

						if let Some(e) = line.strip_prefix("event:") {
							event = Some(e.trim().to_string());
						} else if let Some(d) = line.strip_prefix("data:") {
							if !data.is_empty() {
								data.push('\n');
							}
							data.push_str(d.trim());
						}
					}

					if let Some(frame_tap) = this.frame_tap.as_mut() {
						frame_tap.on_frame(event.as_deref(), &data);
					}

					// If no data found in this block, poll for the next one
					if data.is_empty() {
						continue;
					}

					let event = event.unwrap_or_else(|| "message".to_string());

					return Poll::Ready(Some(Ok(Event::Message(Message { event, data }))));
				}
				Poll::Ready(Some(Err(e))) => {
					return Poll::Ready(Some(Err(e)));
				}
				Poll::Ready(None) => return Poll::Ready(None),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}
