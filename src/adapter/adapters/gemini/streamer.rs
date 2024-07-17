use crate::adapter::gemini::body_to_gemini_chat_response;
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::webc::WebStream;
use crate::{Error, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct GeminiStream {
	inner: WebStream,
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
}

impl GeminiStream {
	pub fn new(inner: WebStream) -> Self {
		GeminiStream { inner, done: false }
	}
}

// Implement futures::Stream for InterStream<GeminiStream>
impl futures::Stream for GeminiStream {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(raw_message)) => {
					// This is the message sent by the WebStream in mode PrettyJsonArray
					// - `[` document start
					// - `{...}` block
					// - `]` document
					let inter_event = match raw_message.as_str() {
						"[" => InterStreamEvent::Start,
						"]" => InterStreamEvent::End(InterStreamEnd::default()),
						block_string => {
							let json_block =
								match serde_json::from_str::<Value>(block_string).map_err(Error::StreamParse) {
									Ok(json_block) => json_block,
									Err(err) => {
										eprintln!("Gemini Adapter Stream Error: {}", err);
										return Poll::Ready(Some(Err(err)));
									}
								};
							let gemini_response = match body_to_gemini_chat_response(json_block) {
								Ok(gemini_response) => gemini_response,
								Err(err) => {
									eprintln!("Gemini Adapter Stream Error: {}", err);
									return Poll::Ready(Some(Err(err)));
								}
							};
							if let Some(text) = gemini_response.content {
								InterStreamEvent::Chunk(text)
							} else {
								continue;
							}
						}
					};

					return Poll::Ready(Some(Ok(inter_event)));
				}
				Some(Err(err)) => {
					println!("Gemini Adapter Stream Error: {}", err);
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
