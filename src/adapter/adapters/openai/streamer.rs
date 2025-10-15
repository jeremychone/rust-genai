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
									AdapterKind::DeepSeek
									| AdapterKind::Zhipu
									| AdapterKind::Fireworks
									| AdapterKind::Together => {
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
						else if let Ok(delta_tool_calls) = first_choice.x_take::<Value>("/delta/tool_calls")
							&& delta_tool_calls != Value::Null
						{
							// Check if there's a tool call in the delta
							if let Some(delta_tool_calls) = delta_tool_calls.as_array()
								&& !delta_tool_calls.is_empty()
							{
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
										let calls = self.captured_data.tool_calls.get_or_insert_with(Vec::new);
										tool_call = capture_tool_call_chunk(
											calls,
											index as usize,
											tool_call,
											fn_arguments.clone(),
											&arguments,
										);
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
						else if let Ok(Some(content)) = first_choice.x_take::<Option<String>>("/delta/content") {
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
						else if let Ok(Some(reasoning_content)) =
							first_choice.x_take::<Option<String>>("/delta/reasoning_content")
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

fn capture_tool_call_chunk(
	calls: &mut Vec<crate::chat::ToolCall>,
	index: usize,
	incoming: crate::chat::ToolCall,
	fn_arguments: Value,
	raw_arguments: &str,
) -> crate::chat::ToolCall {
	if calls.len() <= index {
		calls.resize_with(index + 1, || crate::chat::ToolCall {
			call_id: String::new(),
			fn_name: String::new(),
			fn_arguments: Value::Null,
		});
	}

	let captured_call = &mut calls[index];

	if captured_call.call_id.is_empty() {
		captured_call.call_id = incoming.call_id.clone();
	}

	if captured_call.fn_name.is_empty() {
		captured_call.fn_name = incoming.fn_name.clone();
	}

	if captured_call.fn_arguments.is_null() {
		captured_call.fn_arguments = fn_arguments.clone();
	} else if captured_call.fn_arguments.is_string() {
		if let Some(existing) = captured_call.fn_arguments.as_str() {
			let mut combined = String::with_capacity(existing.len() + raw_arguments.len());
			combined.push_str(existing);
			combined.push_str(raw_arguments);

			captured_call.fn_arguments = serde_json::from_str(&combined).unwrap_or(Value::String(combined));
		}
	} else if let (Value::Object(existing), Value::Object(new)) =
		(&mut captured_call.fn_arguments, fn_arguments.clone())
	{
		existing.extend(new);
	} else {
		captured_call.fn_arguments = fn_arguments;
	}

	captured_call.clone()
}

#[cfg(test)]
mod tests {
	use super::capture_tool_call_chunk;
	use crate::chat::ToolCall;
	use serde_json::{Value, json};

	fn tool_call_template(id: &str) -> ToolCall {
		ToolCall {
			call_id: id.to_string(),
			fn_name: "test".to_string(),
			fn_arguments: Value::Null,
		}
	}

	#[test]
	fn merges_string_chunks_into_object() {
		let mut calls = Vec::new();
		let template = tool_call_template("call_0");

		let first = capture_tool_call_chunk(
			&mut calls,
			0,
			template.clone(),
			Value::String("{\"unit\":".to_string()),
			"{\"unit\":",
		);
		assert!(first.fn_arguments.is_string());
		assert_eq!(calls.len(), 1);

		let second = capture_tool_call_chunk(
			&mut calls,
			0,
			template.clone(),
			Value::String("\"celsius\"}".to_string()),
			"\"celsius\"}",
		);
		let args = second.fn_arguments.as_object().expect("object after merge");
		assert_eq!(args.get("unit").and_then(|v| v.as_str()), Some("celsius"));
	}

	#[test]
	fn extends_object_arguments() {
		let mut calls = Vec::new();
		let template = tool_call_template("call_0");

		let _ = capture_tool_call_chunk(
			&mut calls,
			0,
			template.clone(),
			Value::String("{\"unit\":".to_string()),
			"{\"unit\":",
		);
		let _ = capture_tool_call_chunk(
			&mut calls,
			0,
			template.clone(),
			Value::String("\"celsius\"}".to_string()),
			"\"celsius\"}",
		);
		let merged = capture_tool_call_chunk(
			&mut calls,
			0,
			template.clone(),
			json!({ "location": "Paris" }),
			"{\"location\":\"Paris\"}",
		);

		let args = merged.fn_arguments.as_object().expect("object");
		assert_eq!(args.get("unit").and_then(|v| v.as_str()), Some("celsius"));
		assert_eq!(args.get("location").and_then(|v| v.as_str()), Some("Paris"));
	}

	#[test]
	fn grows_call_buffer_for_new_index() {
		let mut calls = Vec::new();
		let template = tool_call_template("call_2");

		let merged = capture_tool_call_chunk(&mut calls, 2, template.clone(), json!({ "foo": 1 }), "{\"foo\":1}");

		assert_eq!(calls.len(), 3);
		assert_eq!(merged.call_id, "call_2");
		let args = merged.fn_arguments.as_object().expect("object");
		assert_eq!(args.get("foo").and_then(|v| v.as_i64()), Some(1));
	}
}
