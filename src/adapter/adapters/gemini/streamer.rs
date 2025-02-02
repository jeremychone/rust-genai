use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::gemini::{GeminiAdapter, GeminiChatResponse};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::ChatOptionsSet;
use crate::webc::WebStream;
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct GeminiStreamer {
	inner: WebStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to not poll the EventSource after a MessageStop event.
	done: bool,
	captured_data: StreamerCapturedData,
}

impl GeminiStreamer {
	pub fn new(inner: WebStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}
}

// Implement futures::Stream for InterStream<GeminiStream>
impl futures::Stream for GeminiStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(item) = Pin::new(&mut self.inner).poll_next(cx) {
			match item {
				Some(Ok(raw_message)) => {
					// This is the message sent by the WebStream in PrettyJsonArray mode.
					// - `[` document start
					// - `{...}` block
					// - `]` document end
					let inter_event = match raw_message.as_str() {
						"[" => InterStreamEvent::Start,
						"]" => {
							let inter_stream_end = InterStreamEnd {
								captured_usage: self.captured_data.usage.take(),
								captured_content: self.captured_data.content.take(),
								captured_reasoning_content: self.captured_data.reasoning_content.take(),
							};

							InterStreamEvent::End(inter_stream_end)
						}
						block_string => {
							// -- Parse the block to JSON
							let json_block = match serde_json::from_str::<Value>(block_string).map_err(|serde_error| {
								Error::StreamParse {
									model_iden: self.options.model_iden.clone(),
									serde_error,
								}
							}) {
								Ok(json_block) => json_block,
								Err(err) => {
									eprintln!("Gemini Adapter Stream Error: {}", err);
									return Poll::Ready(Some(Err(err)));
								}
							};

							// -- Extract the Gemini Response
							let gemini_response =
								match GeminiAdapter::body_to_gemini_chat_response(&self.options.model_iden, json_block)
								{
									Ok(gemini_response) => gemini_response,
									Err(err) => {
										eprintln!("Gemini Adapter Stream Error: {}", err);
										return Poll::Ready(Some(Err(err)));
									}
								};

							let GeminiChatResponse { content, usage } = gemini_response;

							// -- Send Chunk event
							if let Some(content) = content {
								// Capture content
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut c) => c.push_str(&content),
										None => self.captured_data.content = Some(content.clone()),
									}
								}

								// NOTE: Apparently in the Gemini API, all events have cumulative usage,
								//       meaning each message seems to include the tokens for all previous streams.
								//       Thus, we do not need to add it; we only need to replace captured_data.usage with the latest one.
								//       See https://twitter.com/jeremychone/status/1813734565967802859 for potential additional information.
								if self.options.capture_usage {
									self.captured_data.usage = Some(usage);
								}

								InterStreamEvent::Chunk(content)
							} else {
								continue;
							}
						}
					};

					return Poll::Ready(Some(Ok(inter_event)));
				}
				Some(Err(err)) => {
					println!("Gemini Adapter Stream Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
					})));
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
