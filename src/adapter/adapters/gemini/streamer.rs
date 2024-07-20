use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::gemini::{GeminiAdapter, GeminiChatResponse};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::ChatRequestOptionsSet;
use crate::webc::WebStream;
use crate::{Error, ModelInfo, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct GeminiStreamer {
	inner: WebStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl GeminiStreamer {
	pub fn new(inner: WebStream, model_info: ModelInfo, options_set: ChatRequestOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_info, options_set),
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
					// This is the message sent by the WebStream in mode PrettyJsonArray
					// - `[` document start
					// - `{...}` block
					// - `]` document
					let inter_event = match raw_message.as_str() {
						"[" => InterStreamEvent::Start,
						"]" => {
							let inter_stream_end = InterStreamEnd {
								captured_usage: self.captured_data.usage.take(),
								captured_content: self.captured_data.content.take(),
							};

							InterStreamEvent::End(inter_stream_end)
						}
						block_string => {
							// -- Parse the block to json
							let json_block = match serde_json::from_str::<Value>(block_string).map_err(|serde_error| {
								Error::StreamParse {
									model_info: self.options.model_info.clone(),
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
								match GeminiAdapter::body_to_gemini_chat_response(&self.options.model_info, json_block)
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
								// capture content
								if self.options.capture_content {
									match self.captured_data.content {
										Some(ref mut c) => c.push_str(&content),
										None => self.captured_data.content = Some(content.clone()),
									}
								}

								// NOTE: Apparently in Gemini API, all event get a usage but their are cumulative
								//       meaning the each message seems to have the tokens for all of the previous stream.
								//       So, we do not need ot add it, just to replace the captured_data.usage with the latest one.
								//       See https://twitter.com/jeremychone/status/1813734565967802859 for potential additional information
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
						model_info: self.options.model_info.clone(),
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
