use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::AdapterKind;
use crate::chat::ChatOptionsSet;
use crate::{Error, ModelIden, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct OpenAIStreamer {
	inner: EventSource,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl OpenAIStreamer {
	// TODO: Problem - need the ChatOptions `.capture_content` and `.capture_usage`
	pub fn new(inner: EventSource, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}
}

impl futures::Stream for OpenAIStreamer {
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
					// According to OpenAI Spec, this is the end message
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
							captured_content: self.captured_data.content.take(),
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// -- Other Content Messages
					let adapter_kind = self.options.model_iden.adapter_kind;
					// Parse to get the choice
					let mut message_data: Value =
						serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
							model_iden: self.options.model_iden.clone(),
							serde_error,
						})?;
					let first_choice: Option<Value> = message_data.x_take("/choices/0").ok();

					// If we have a first choice, then it's a normal message
					if let Some(mut first_choice) = first_choice {
						// If finish_reason exists, it's the end of this choice.
						// Since we support only a single choice, we can proceed,
						// as there might be other messages, and the last one contains data: `[DONE]`
						if let Some(_finish_reason) = first_choice.x_take::<Option<String>>("finish_reason")? {
							// NOTE: For Groq, the usage is captured when finish_reason indicates stopping, and in the `/x_groq/usage`
							if matches!(adapter_kind, AdapterKind::Groq) && self.options.capture_usage {
								let usage = message_data
									.x_take("/x_groq/usage")
									.map(OpenAIAdapter::into_usage)
									.unwrap_or_default(); // permissive for now
								self.captured_data.usage = Some(usage)
							}
							continue;
						}
						// If there is no finish_reason but there is some content, we can get the delta content and send the Internal Stream Event
						else if let Some(content) = first_choice.x_take::<Option<String>>("/delta/content")? {
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
						else {
							// TODO: use tracing debug
							println!("EMPTY CHOICE CONTENT");
						}
					}
					// -- Usage message
					else {
						// If it's not Groq, then the usage is captured at the end when choices are empty or null

						if !matches!(adapter_kind, AdapterKind::Groq)
							&& self.captured_data.usage.is_none()
							&& self.options.capture_usage
						{
							let usage = message_data.x_take("usage").map(OpenAIAdapter::into_usage).unwrap_or_default(); // permissive for now
							self.captured_data.usage = Some(usage); // permissive for now
						}
					}
				}
				Some(Err(err)) => {
					println!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::ReqwestEventSource(err))));
				}
				None => {
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}
