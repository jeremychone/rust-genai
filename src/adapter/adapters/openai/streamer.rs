use crate::adapter::AdapterKind;
use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::adapter::openai::OpenAIAdapter;
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
						// TODO: Needs to clarify wh for usage we do not adopt the same strategy from captured content below
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

					let adapter_kind = self.options.model_iden.adapter_kind;

					// If we have a first choice, then it's a normal message
					if let Some(mut first_choice) = first_choice {
						// -- Finish Reason
						// If finish_reason exists, it's the end of this choice.
						// Since we support only a single choice, we can proceed,
						// as there might be other messages, and the last one contains data: `[DONE]`
						// NOTE: xAI has no `finish_reason` when not finished, so, need to just account for both null/absent
						if let Ok(_finish_reason) = first_choice.x_take::<String>("finish_reason") {
							// NOTE: For Groq, the usage is captured when finish_reason indicates stopping, and in the `/x_groq/usage`
							if self.options.capture_usage {
								match adapter_kind {
									AdapterKind::Groq => {
										let usage = message_data
											.x_take("/x_groq/usage")
											.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
											.unwrap_or_default(); // permissive for now
										self.captured_data.usage = Some(usage)
									}
									AdapterKind::DeepSeek => {
										let usage = message_data
											.x_take("usage")
											.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
											.unwrap_or_default();
										self.captured_data.usage = Some(usage)
									},
									AdapterKind::Zhipu => {
										let usage = message_data
											.x_take("usage")
											.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
											.unwrap_or_default();
										self.captured_data.usage = Some(usage)
									}
									_ => (), // do nothing, will be captured the OpenAI way
								}
							}

							continue;
						}
						// -- Tool Call
						else if let Ok(delta_tool_calls) = first_choice.x_take::<Value>("/delta/tool_calls") {
							// Check if there's a tool call in the delta
							if delta_tool_calls.is_array() && !delta_tool_calls.as_array().unwrap().is_empty() {
								// Extract the first tool call object as a mutable value
								let mut tool_call_obj = delta_tool_calls[0].clone();

								// Extract tool call data
								if let (Ok(index), Ok(mut function)) = (
									tool_call_obj.x_take::<u32>("index"),
									tool_call_obj.x_take::<Value>("function"),
								) {
									let call_id = tool_call_obj
										.x_take::<String>("id")
										.unwrap_or_else(|_| format!("call_{index}"));
									let fn_name = function.x_take::<String>("name").unwrap_or_default();
									let arguments = function.x_take::<String>("arguments").unwrap_or_default();
									// Create the tool call
									let fn_arguments = serde_json::from_str(&arguments)
										.unwrap_or(serde_json::Value::String(arguments.clone()));
									let mut tool_call = crate::chat::ToolCall {
										call_id,
										fn_name,
										fn_arguments: fn_arguments.clone(),
									};

									// Capture the tool call if enabled
									if self.options.capture_tool_calls {
										match &mut self.captured_data.tool_calls {
											Some(calls) => {
												self.captured_data.tool_calls = Some({
													// When fn_arguments can not be parsed, we need to append the arguments to the existing fn_arguments as json string
													let mut captured_fn_argments = String::new();
													if calls[index as usize].fn_arguments.is_string() {
														captured_fn_argments.push_str(
															calls[index as usize].fn_arguments.as_str().unwrap_or(""),
														);
														captured_fn_argments.push_str(&arguments);
													}
													let fn_arguments = serde_json::from_str(&captured_fn_argments)
														.unwrap_or(serde_json::Value::String(
															captured_fn_argments.clone(),
														));
													calls[index as usize].fn_arguments = fn_arguments.clone();
													tool_call = calls[index as usize].clone();
													calls.to_vec()
												})
											}
											None => self.captured_data.tool_calls = Some(vec![tool_call.clone()]),
										}
									}

									// Return the ToolCallChunk event
									return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tool_call))));
								}
							}
							// No valid tool call found, continue to next message
							continue;
						}
						// -- Content
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
						// -- Reasoning Content
						else if let Some(reasoning_content) =
							first_choice.x_take::<Option<String>>("/delta/reasoning_content")?
						{
							// Add to the captured_content if chat options allow it
							if self.options.capture_reasoning_content {
								match self.captured_data.reasoning_content {
									Some(ref mut c) => c.push_str(&reasoning_content),
									None => self.captured_data.reasoning_content = Some(reasoning_content.clone()),
								}
							}

							// Return the Event
							return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(reasoning_content))));
						}
						// If we do not have content, then log a trace message
						else {
							// TODO: use tracing debug
							tracing::warn!("EMPTY CHOICE CONTENT");
						}
					}
					// -- Usage message
					else {
						// If it's not Groq, xAI, DeepSeek the usage is captured at the end when choices are empty or null
						if !matches!(adapter_kind, AdapterKind::Groq)
							&& !matches!(adapter_kind, AdapterKind::DeepSeek)
							&& self.captured_data.usage.is_none() // this might be redundant
							&& self.options.capture_usage
						{
							// permissive for now
							let usage = message_data
								.x_take("usage")
								.map(|v| OpenAIAdapter::into_usage(adapter_kind, v))
								.unwrap_or_default();
							self.captured_data.usage = Some(usage);
						}
					}
				}
				Some(Err(err)) => {
					tracing::error!("Error: {}", err);
					return Poll::Ready(Some(Err(Error::ReqwestEventSource(err.into()))));
				}
				None => {
					return Poll::Ready(None);
				}
			}
		}
		Poll::Pending
	}
}
