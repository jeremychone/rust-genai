use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::ChatOptionsSet;
use crate::{Error, ModelIden, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct CerebrasStreamer {
	inner: EventSource,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl CerebrasStreamer {
	pub fn new(inner: EventSource, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}
}

impl futures::Stream for CerebrasStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			// The last poll was definitely the end, so end the stream.
			// This will prevent triggering a stream ended error
			return Poll::Ready(None);
		}

		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					// -- End Message
					// Cerebras may not send [DONE] like OpenAI, so we need to handle stream ending differently
					if message.data == "[DONE]" {
						self.done = true;

						// -- Build the usage and captured_content
						let captured_usage = if self.options.capture_usage {
							self.captured_data.usage.take()
						} else {
							None
						};

						let inter_stream_end = InterStreamEnd {
							captured_usage,
							captured_text_content: self.captured_data.content.take(),
							captured_reasoning_content: self.captured_data.reasoning_content.take(),
							captured_tool_calls: self.captured_data.tool_calls.take(),
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// -- Other Content Messages
					// Parse to get the choice
					let mut message_data: Value =
						serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
							model_iden: self.options.model_iden.clone(),
							serde_error,
						})?;

					let first_choice: Option<Value> = message_data.x_take("/choices/0").ok();

					// If we have a first choice, then it's a normal message
					if let Some(mut first_choice) = first_choice {
						// -- Finish Reason
						// If finish_reason exists, it's the end of this choice.
						// Since we support only a single choice, we can proceed,
						// as there might be other messages, and the last one contains data: `[DONE]`
						// NOTE: Cerebras may have different finish_reason behavior
						if let Ok(_finish_reason) = first_choice.x_take::<String>("finish_reason") {
							// For Cerebras, we capture usage when we see finish_reason
							if self.options.capture_usage
								&& let Ok(usage) = message_data.x_take("usage")
								&& let Ok(usage) = serde_json::from_value(usage)
							{
								self.captured_data.usage = Some(usage);
							}
						}

						// -- Content
						if let Ok(Some(content)) = first_choice.x_take::<Option<String>>("/delta/content") {
							// Add to the captured_content if chat options allow it
							if self.options.capture_content {
								match self.captured_data.content {
									Some(ref mut c) => c.push_str(&content),
									None => self.captured_data.content = Some(content.clone()),
								}
							}

							// Return the Event
							return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(content))));
						}
						// If we do not have content, then log a trace message
						// TODO: use tracing debug
						tracing::warn!("EMPTY CHOICE CONTENT");
					}
					// -- Usage message
					else {
						// For Cerebras, capture usage when choices are empty or null
						if self.captured_data.usage.is_none() // this might be redundant
							&& self.options.capture_usage
							&& let Ok(usage) = message_data.x_take("usage")
							&& let Ok(usage) = serde_json::from_value(usage)
						{
							self.captured_data.usage = Some(usage);
						}
					}
				}
				Some(Err(err)) => {
					// Cerebras sometimes ends the stream with a StreamEnded error instead of clean None
					// We'll treat this as a normal stream end
					tracing::debug!("Cerebras stream ended with error (this is expected): {}", err);
					self.done = true;

					// -- Build the usage and captured_content
					let captured_usage = if self.options.capture_usage {
						self.captured_data.usage.take()
					} else {
						None
					};

					let inter_stream_end = InterStreamEnd {
						captured_usage,
						captured_text_content: self.captured_data.content.take(),
						captured_reasoning_content: self.captured_data.reasoning_content.take(),
						captured_tool_calls: self.captured_data.tool_calls.take(),
					};

					return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
				}
				None => {
					// Cerebras stream ends without [DONE], so we need to create the StreamEnd event here
					self.done = true;

					// -- Build the usage and captured_content
					let captured_usage = if self.options.capture_usage {
						self.captured_data.usage.take()
					} else {
						None
					};

					let inter_stream_end = InterStreamEnd {
						captured_usage,
						captured_text_content: self.captured_data.content.take(),
						captured_reasoning_content: self.captured_data.reasoning_content.take(),
						captured_tool_calls: self.captured_data.tool_calls.take(),
					};

					return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
				}
			}
		}
		Poll::Pending
	}
}
