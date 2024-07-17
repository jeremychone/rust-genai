use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatRequestOptionsSet, MetaUsage};
use crate::utils::x_value::XValue;
use crate::{Error, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct OpenAIStreamer {
	inner: EventSource,
	options: OpenAiStreamOptions,

	// -- Set by the poll_next
	/// Flag to not poll the EventSource after a MessageStop event
	done: bool,
	// The eventual captured usage json value
	// `{prompt_tokens: number, complete_tokens: number, total_tokens: number}`
	captured_usage: Option<Value>,
	captured_content: Option<String>,
}

impl OpenAIStreamer {
	// TODO: Problen need the ChatRequestOptions `.capture_content` `.capture_usage`
	pub fn new(inner: EventSource, options_set: ChatRequestOptionsSet<'_, '_>) -> Self {
		OpenAIStreamer {
			inner,
			options: options_set.into(),
			done: false,
			captured_usage: None,
			captured_content: None,
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
							self.captured_usage.take().map(|mut usage_value| {
								// Note: Here we do not want to fail
								let input_tokens: Option<i32> = usage_value.x_take("prompt_tokens").ok();
								let output_tokens: Option<i32> = usage_value.x_take("completion_tokens").ok();
								let total_tokens: Option<i32> = usage_value.x_take("total_tokens").ok();
								MetaUsage {
									input_tokens,
									output_tokens,
									total_tokens,
								}
							})
						} else {
							None
						};

						let inter_stream_end = InterStreamEnd {
							captured_usage,
							captured_content: self.captured_content.take(),
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// -- Other Content Messages
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
							if self.options.capture_usage {
								self.captured_usage = message_data.x_take("/x_groq/usage").ok(); // premissive for now
							}
							continue;
						}
						// If not finish_reason and some content, we can get the delta content and send the Internal Stream Event
						else if let Some(content) = first_choice.x_take::<Option<String>>("/delta/content")? {
							// add to the captured_content if chat options say so
							if self.options.capture_content {
								match self.captured_content {
									Some(ref mut c) => c.push_str(&content),
									None => self.captured_content = Some(content.clone()),
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
						// Note: With OpenAI the usage in the last message when there is no choices anymore.
						//       Here we make sure we do not override the potential groq usage captured above.
						//       Later, the OpenAIStreamer might take the AdapterKind to do those branching
						if self.captured_usage.is_none() && self.options.capture_usage {
							self.captured_usage = message_data.x_take("/usage").ok(); // premissive for now
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

// region:    --- OpenAiStreamOptions

#[derive(Debug, Default)]
pub struct OpenAiStreamOptions {
	capture_content: bool,
	capture_usage: bool,
}

impl From<ChatRequestOptionsSet<'_, '_>> for OpenAiStreamOptions {
	fn from(options_set: ChatRequestOptionsSet) -> Self {
		OpenAiStreamOptions {
			capture_content: options_set.capture_content().unwrap_or(false),
			capture_usage: options_set.capture_usage().unwrap_or(false),
		}
	}
}

// endregion: --- OpenAiStreamOptions
