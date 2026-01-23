use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, ToolCall, Usage};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde_json::{Map, Value};
use std::pin::Pin;
use std::task::{Context, Poll};
use value_ext::JsonValueExt;

pub struct AnthropicStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,

	// -- Set by the poll_next
	/// Flag to prevent polling the EventSource after a MessageStop event
	done: bool,

	captured_data: StreamerCapturedData,
	in_progress_block: InProgressBlock,
}

enum InProgressBlock {
	Text,
	ToolUse { id: String, name: String, input: String },
	Thinking,
}

impl AnthropicStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			in_progress_block: InProgressBlock::Text,
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
							let mut data: Value =
								serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
									model_iden: self.options.model_iden.clone(),
									serde_error,
								})?;

							match data.x_get_str("/content_block/type") {
								Ok("text") => self.in_progress_block = InProgressBlock::Text,
								Ok("thinking") => self.in_progress_block = InProgressBlock::Thinking,
								Ok("tool_use") => {
									self.in_progress_block = InProgressBlock::ToolUse {
										id: data.x_take("/content_block/id")?,
										name: data.x_take("/content_block/name")?,
										input: String::new(),
									};
								}
								Ok(txt) => {
									tracing::warn!("unhandled content type: {txt}");
								}
								Err(e) => {
									tracing::error!("{e:?}");
								}
							}

							continue;
						}
						"content_block_delta" => {
							let mut data: Value =
								serde_json::from_str(&message.data).map_err(|serde_error| Error::StreamParse {
									model_iden: self.options.model_iden.clone(),
									serde_error,
								})?;

							match &mut self.in_progress_block {
								InProgressBlock::Text => {
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
								InProgressBlock::ToolUse { input, .. } => {
									input.push_str(data.x_get_str("/delta/partial_json")?);
									continue;
								}
								InProgressBlock::Thinking => {
									if let Ok(thinking) = data.x_take::<String>("/delta/thinking") {
										// Add to the captured_thinking if chat options say so
										if self.options.capture_reasoning_content {
											match self.captured_data.reasoning_content {
												Some(ref mut r) => r.push_str(&thinking),
												None => self.captured_data.reasoning_content = Some(thinking.clone()),
											}
										}

										return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(thinking))));
									} else if let Ok(signature) = data.x_take::<String>("/delta/signature") {
										return Poll::Ready(Some(Ok(InterStreamEvent::ThoughtSignatureChunk(
											signature,
										))));
									} else {
										// If it is thinking but no thinking or signature field, we log and skip.
										tracing::warn!(
											"content_block_delta for thinking block but no thinking or signature found: {data:?}"
										);
										continue;
									}
								}
							}
						}
						"content_block_stop" => {
							match std::mem::replace(&mut self.in_progress_block, InProgressBlock::Text) {
								InProgressBlock::ToolUse { id, name, input } => {
									let fn_arguments = if input.is_empty() {
										Value::Object(Map::new())
									} else {
										serde_json::from_str(&input)?
									};

									let tc = ToolCall {
										call_id: id,
										fn_name: name,
										fn_arguments,
										thought_signatures: None,
									};

									// Add to the captured_tool_calls if chat options say so
									if self.options.capture_tool_calls {
										match self.captured_data.tool_calls {
											Some(ref mut t) => t.push(tc.clone()),
											None => self.captured_data.tool_calls = Some(vec![tc.clone()]),
										}
									}

									return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tc))));
								}
								_ => {
									// no-op for remaining block types
								}
							}

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
									if usage.prompt_tokens.is_some() || usage.completion_tokens.is_some() {
										usage.total_tokens = Some(
											usage.prompt_tokens.unwrap_or(0) + usage.completion_tokens.unwrap_or(0),
										);
									}
									usage
								})
							} else {
								None
							};

							let inter_stream_end = InterStreamEnd {
								captured_usage,
								captured_text_content: self.captured_data.content.take(),
								captured_reasoning_content: self.captured_data.reasoning_content.take(),
								captured_tool_calls: self.captured_data.tool_calls.take(),
								captured_thought_signatures: None,
							};

							// TODO: Need to capture the data as needed
							return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
						}

						"ping" => continue, // Loop to the next event
						other => tracing::warn!("UNKNOWN MESSAGE TYPE: {other}"),
					}
				}
				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
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
				tracing::debug!(
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
					.get_or_insert(Usage::default())
					.prompt_tokens
					.get_or_insert(0);
				*val += input_tokens;
			}

			if let Ok(output_tokens) = data.x_get::<i32>(output_path) {
				let val = self
					.captured_data
					.usage
					.get_or_insert(Usage::default())
					.completion_tokens
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
