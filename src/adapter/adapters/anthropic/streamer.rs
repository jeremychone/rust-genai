use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, MetaUsage};
use crate::{Error, ModelIden, Result};
use reqwest_eventsource::{Event, EventSource};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct AnthropicStreamer {
	inner: EventSource,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,
	captured_data: StreamerCapturedData,
}

impl AnthropicStreamer {
	pub fn new(inner: EventSource, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
		}
	}
}

impl futures::Stream for AnthropicStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			// NOTE: At this point, we capture more events than needed for genai::StreamItem, but it serves as documentation.
			match event {
				Some(Ok(Event::Open)) => return Poll::Ready(Some(Ok(InterStreamEvent::Start))),
				Some(Ok(Event::Message(message))) => {
					let message_type = message.event.as_str();

					match message_type {
						"message_start" => {
							self.capture_usage(message_type, &message.data)?;
							continue;
						}
						"message_delta" => {
							self.capture_usage(message_type, &message.data)?;
							continue;
						}
						"content_block_start" => {
							continue;
						}
						"content_block_delta" => {
							let mut data: Value =
								serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
									model_iden: self.options.model_iden.clone(),
									serde_error,
								})?;
							let content: String = data.x_take("/delta/text")?;

							// Add to the captured_content if chat options say so
							if self.options.capture_content {
								match self.captured_data.content {
									Some(ref mut c) => c.push_str(&content),
									None => self.captured_data.content = Some(content.clone()),
								}
							}

							return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(content))));
						}
						"content_block_stop" => {
							continue;
						}
						// -- END MESSAGE
						"message_stop" => {
							// Ensure we do not poll the EventSource anymore on the next poll.
							// NOTE: This way, the last MessageStop event is still sent,
							//       but then, on the next poll, it will be stopped.
							self.done = true;

							// Capture the usage
							let captured_usage = if self.options.capture_usage {
								self.captured_data.usage.take().map(|mut usage| {
									// Compute the total if any of input/output are not null
									if usage.input_tokens.is_some() || usage.output_tokens.is_some() {
										usage.total_tokens =
											Some(usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0));
									}
									usage
								})
							} else {
								None
							};

							let inter_stream_end = InterStreamEnd {
								captured_usage,
								captured_content: self.captured_data.content.take(),
								captured_reasoning_content: self.captured_data.reasoning_content.take(),
							};

							// TODO: Need to capture the data as needed
							return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
						}

						"ping" => continue, // Loop to the next event
						other => println!("UNKNOWN MESSAGE TYPE: {other}"),
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

// Support
impl AnthropicStreamer {
	fn capture_usage(&mut self, message_type: &str, message_data: &str) -> Result<()> {
		if self.options.capture_usage {
			let data = self.parse_message_data(message_data)?;
			// TODO: Might want to exit early if usage is not found

			let (input_path, output_path) = if message_type == "message_start" {
				("/message/usage/input_tokens", "/message/usage/output_tokens")
			} else if message_type == "message_delta" {
				("/usage/input_tokens", "/usage/output_tokens")
			} else {
				// TODO: Use tracing
				println!(
					"TRACING DEBUG - Anthropic message type not supported for input/output tokens: {message_type}"
				);
				return Ok(()); // For now permissive
			};

			// -- Capture/Add the eventual input_tokens
			// NOTE: Permissive on this one; if an error occurs, treat it as nonexistent (for now)
			if let Ok(input_tokens) = data.x_get::<i32>(input_path) {
				let val = self
					.captured_data
					.usage
					.get_or_insert(MetaUsage::default())
					.input_tokens
					.get_or_insert(0);
				*val += input_tokens;
			}

			if let Ok(output_tokens) = data.x_get::<i32>(output_path) {
				let val = self
					.captured_data
					.usage
					.get_or_insert(MetaUsage::default())
					.output_tokens
					.get_or_insert(0);
				*val += output_tokens;
			}
		}

		Ok(())
	}

	/// Simple wrapper for now, with the corresponding map_err.
	/// Might have more logic later.
	fn parse_message_data(&self, payload: &str) -> Result<Value> {
		serde_json::from_str(payload).map_err(|serde_error| Error::StreamParse {
			model_iden: self.options.model_iden.clone(),
			serde_error,
		})
	}
}
