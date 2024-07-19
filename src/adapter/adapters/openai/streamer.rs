use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::adapter::openai::OpenAIAdapter;
use crate::adapter::AdapterKind;
use crate::chat::ChatRequestOptionsSet;
use crate::support::value_ext::ValueExt;
use crate::{Error, ModelInfo, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct OpenAIStreamer {
	inner: EventSource,
	options: StreamerOptions,
	// Because the OpenAI Adapter/Streamer, the model_info.adapter_kind might be different than OpenAI
	model_info: ModelInfo,

	// -- Set by the poll_next
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl OpenAIStreamer {
	// TODO: Problen need the ChatRequestOptions `.capture_content` `.capture_usage`
	pub fn new(inner: EventSource, model_info: ModelInfo, options_set: ChatRequestOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			model_info,
			options: options_set.into(),
			captured_data: Default::default(),
		}
	}
}

impl futures::Stream for OpenAIStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			// The last poll was the end for sure, so, end the stream.
			// This will avoid doing the stream ended error
			return Poll::Ready(None);
		}
		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					// -- End Message
					// Per OpenAI Spec, this is the end message
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
					let adapter_kind = self.model_info.adapter_kind;
					// parse to get the choice
					let mut message_data: Value = serde_json::from_str(&message.data).map_err(Error::StreamParse)?;
					let first_choice: Option<Value> = message_data.x_take("/choices/0").ok();

					// if we have a first choice, then, normal message
					if let Some(mut first_choice) = first_choice {
						// If finish_reason, it's the end of this choice,
						// Since we support only single choice, we are good, and we just continue because
						// there might be other message, and the end one is with data: `[DONE]`
						if let Some(_finish_reason) = first_choice.x_take::<Option<String>>("finish_reason")? {
							// NOTE: For Groq, the usage is captured in the when finish_reason stop, and in the `/x_groq/usage`
							if matches!(adapter_kind, AdapterKind::Groq) && self.options.capture_usage {
								let usage = message_data
									.x_take("/x_groq/usage")
									.map(OpenAIAdapter::into_usage)
									.unwrap_or_default(); // premissive for now
								self.captured_data.usage = Some(usage)
							}
							continue;
						}
						// If not finish_reason and some content, we can get the delta content and send the Internal Stream Event
						else if let Some(content) = first_choice.x_take::<Option<String>>("/delta/content")? {
							// add to the captured_content if chat options say so
							if self.options.capture_content {
								match self.captured_data.content {
									Some(ref mut c) => c.push_str(&content),
									None => self.captured_data.content = Some(content.clone()),
								}
							}

							// return the Event
							return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(content))));
						}
						// If we do not have content, then, do a trace message
						else {
							// TODO: use tracing debug
							println!("EMPTY CHOICE CONTENT");
						}
					}
					// -- Usage message
					else {
						// If not Groq, then the usage is at the end when choices is empty/null

						if !matches!(adapter_kind, AdapterKind::Groq)
							&& self.captured_data.usage.is_none()
							&& self.options.capture_usage
						{
							let usage = message_data.x_take("usage").map(OpenAIAdapter::into_usage).unwrap_or_default(); // premissive for now
							self.captured_data.usage = Some(usage); // premissive for now
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
