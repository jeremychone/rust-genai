use crate::error::{BoxError, Error as GenaiError};
use crate::webc::StreamRequest as RequestBuilder;
use crate::webc::{FrameTap, WebStream};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// How much of a body that is not SSE is kept for `Error::StreamResponseNotSse`.
const NOT_SSE_BODY_MAX_LEN: usize = 4096;
/// Appended to the kept body when the cap cut it.
const NOT_SSE_BODY_TRUNCATED: &str = "\n[truncated]";

/// Simple EventSource stream implementation that uses WebStream as a foundation.
pub struct EventSourceStream {
	inner: WebStream,
	opened: bool,
	frame_tap: Option<FrameTap>,
	/// Set by the first event with data. Until then, blocks with lines that are not SSE are kept
	/// in `not_sse_body`, so a stream that ends without any event can say what it got instead.
	has_data: bool,
	not_sse_body: String,
	/// Set once the cap cut some of `not_sse_body`.
	not_sse_truncated: bool,
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
			has_data: false,
			not_sse_body: String::new(),
			not_sse_truncated: false,
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

	/// Keeps a block that is not SSE, up to `NOT_SSE_BODY_MAX_LEN` bytes in all.
	fn keep_not_sse(&mut self, raw_event: &str) {
		let separator = if self.not_sse_body.is_empty() { "" } else { "\n\n" };
		let room = NOT_SSE_BODY_MAX_LEN.saturating_sub(self.not_sse_body.len() + separator.len());
		let mut end = room.min(raw_event.len());
		while !raw_event.is_char_boundary(end) {
			end -= 1;
		}
		if end > 0 {
			self.not_sse_body.push_str(separator);
			self.not_sse_body.push_str(&raw_event[..end]);
		}
		self.not_sse_truncated |= end < raw_event.len();
	}

	/// At the end of a stream that had no event, the error for the body that was not SSE, if any.
	fn take_not_sse_error(&mut self) -> Option<BoxError> {
		if self.has_data || self.not_sse_body.is_empty() {
			return None;
		}
		let (status, content_type) = self.inner.response_head()?;
		let mut body = std::mem::take(&mut self.not_sse_body);
		if self.not_sse_truncated {
			body.push_str(NOT_SSE_BODY_TRUNCATED);
		}
		Some(Box::new(GenaiError::StreamResponseNotSse {
			status,
			content_type: content_type.to_string(),
			body,
		}))
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
					let mut has_not_sse_line = false;
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
						} else {
							// Other SSE fields (or a field with no value, like a bare `data`) carry no event here.
							let field = line.split_once(':').map_or(line, |(field, _)| field);
							has_not_sse_line |= !matches!(field, "data" | "event" | "id" | "retry");
						}
					}

					if let Some(frame_tap) = this.frame_tap.as_mut() {
						frame_tap.on_frame(event.as_deref(), &data);
					}

					// If no data found in this block, poll for the next one
					if data.is_empty() {
						if has_not_sse_line && !this.has_data {
							this.keep_not_sse(&raw_event);
						}
						continue;
					}
					if !this.has_data {
						this.has_data = true;
						this.not_sse_body = String::new();
						this.not_sse_truncated = false;
					}

					let event = event.unwrap_or_else(|| "message".to_string());

					return Poll::Ready(Some(Ok(Event::Message(Message { event, data }))));
				}
				Poll::Ready(Some(Err(e))) => {
					return Poll::Ready(Some(Err(e)));
				}
				// A stream that ended without any event, after content that was not SSE, ends with an error.
				Poll::Ready(None) => return Poll::Ready(this.take_not_sse_error().map(Err)),
				Poll::Pending => return Poll::Pending,
			}
		}
	}
}
