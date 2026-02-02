//! Bedrock ConverseStream response streamer
//!
//! Handles the streaming response format from AWS Bedrock's ConverseStream API.
//! The Bedrock streaming format uses Server-Sent Events with JSON payloads.

use crate::adapter::adapters::support::{StreamerCapturedData, StreamerOptions};
use crate::adapter::inter_stream::{InterStreamEnd, InterStreamEvent};
use crate::chat::{ChatOptionsSet, ToolCall, Usage};
use crate::webc::{Event, EventSourceStream};
use crate::{Error, ModelIden, Result};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct BedrockStreamer {
	inner: EventSourceStream,
	options: StreamerOptions,

	// State flags
	done: bool,

	// Captured data for the stream end
	captured_data: StreamerCapturedData,

	// In-progress tracking for tool use blocks
	in_progress_block: InProgressBlock,
}

enum InProgressBlock {
	Text,
	ToolUse { id: String, name: String, input: String },
	Reasoning,
}

impl BedrockStreamer {
	pub fn new(inner: EventSourceStream, model_iden: ModelIden, options_set: ChatOptionsSet<'_, '_>) -> Self {
		Self {
			inner,
			done: false,
			options: StreamerOptions::new(model_iden, options_set),
			captured_data: Default::default(),
			in_progress_block: InProgressBlock::Text,
		}
	}

	/// Parse usage from Bedrock metadata event
	fn parse_usage(&self, data: &Value) -> Option<Usage> {
		let usage = data.get("usage")?;

		let input_tokens = usage.get("inputTokens")?.as_i64()? as i32;
		let output_tokens = usage.get("outputTokens")?.as_i64()? as i32;
		let total_tokens = input_tokens + output_tokens;

		Some(Usage {
			prompt_tokens: Some(input_tokens),
			prompt_tokens_details: None,
			completion_tokens: Some(output_tokens),
			completion_tokens_details: None,
			total_tokens: Some(total_tokens),
		})
	}
}

impl futures::Stream for BedrockStreamer {
	type Item = Result<InterStreamEvent>;

	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		if self.done {
			return Poll::Ready(None);
		}

		while let Poll::Ready(event) = Pin::new(&mut self.inner).poll_next(cx) {
			match event {
				Some(Ok(Event::Open)) => {
					return Poll::Ready(Some(Ok(InterStreamEvent::Start)));
				}
				Some(Ok(Event::Message(message))) => {
					// Bedrock sends events with different types in the event field
					// The data is always JSON
					let data: Value = match serde_json::from_str(&message.data) {
						Ok(d) => d,
						Err(e) => {
							tracing::warn!("Failed to parse Bedrock stream data: {}", e);
							continue;
						}
					};

					// Bedrock stream events are structured differently
					// They come as typed events with the event type in the JSON

					// Handle messageStart event
					if data.get("messageStart").is_some() {
						// Message start doesn't contain content, just continue
						continue;
					}

					// Handle contentBlockStart event
					if let Some(block_start) = data.get("contentBlockStart") {
						if let Some(start) = block_start.get("start") {
							// Check what type of content block is starting
							if start.get("text").is_some() {
								self.in_progress_block = InProgressBlock::Text;
							} else if let Some(tool_use) = start.get("toolUse") {
								let id = tool_use.get("toolUseId").and_then(|v| v.as_str()).unwrap_or("").to_string();
								let name = tool_use.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
								self.in_progress_block = InProgressBlock::ToolUse {
									id,
									name,
									input: String::new(),
								};
							} else if start.get("reasoningContent").is_some() {
								self.in_progress_block = InProgressBlock::Reasoning;
							}
						}
						continue;
					}

					// Handle contentBlockDelta event
					if let Some(block_delta) = data.get("contentBlockDelta") {
						if let Some(delta) = block_delta.get("delta") {
							match &mut self.in_progress_block {
								InProgressBlock::Text => {
									if let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
										let text = text.to_string();

										// Capture content if requested
										if self.options.capture_content {
											match self.captured_data.content {
												Some(ref mut c) => c.push_str(&text),
												None => self.captured_data.content = Some(text.clone()),
											}
										}

										return Poll::Ready(Some(Ok(InterStreamEvent::Chunk(text))));
									}
								}
								InProgressBlock::ToolUse { input, .. } => {
									if let Some(tool_input) = delta.get("toolUse")
										&& let Some(partial) = tool_input.get("input").and_then(|v| v.as_str())
									{
										input.push_str(partial);
									}
								}
								InProgressBlock::Reasoning => {
									if let Some(reasoning) = delta
										.get("reasoningContent")
										.and_then(|v| v.get("text"))
										.and_then(|v| v.as_str())
									{
										let reasoning = reasoning.to_string();

										// Capture reasoning if requested
										if self.options.capture_reasoning_content {
											match self.captured_data.reasoning_content {
												Some(ref mut r) => r.push_str(&reasoning),
												None => self.captured_data.reasoning_content = Some(reasoning.clone()),
											}
										}

										return Poll::Ready(Some(Ok(InterStreamEvent::ReasoningChunk(reasoning))));
									}
								}
							}
						}
						continue;
					}

					// Handle contentBlockStop event
					if data.get("contentBlockStop").is_some() {
						// If we were building a tool use block, emit it now
						match std::mem::replace(&mut self.in_progress_block, InProgressBlock::Text) {
							InProgressBlock::ToolUse { id, name, input } => {
								let fn_arguments: Value = serde_json::from_str(&input).unwrap_or(Value::Null);

								let tool_call = ToolCall {
									call_id: id,
									fn_name: name,
									fn_arguments,
									thought_signatures: None,
								};

								// Capture tool calls if requested
								if self.options.capture_tool_calls {
									match self.captured_data.tool_calls {
										Some(ref mut t) => t.push(tool_call.clone()),
										None => self.captured_data.tool_calls = Some(vec![tool_call.clone()]),
									}
								}

								return Poll::Ready(Some(Ok(InterStreamEvent::ToolCallChunk(tool_call))));
							}
							_ => {
								// No action needed for other block types
							}
						}
						continue;
					}

					// Handle messageStop event
					if data.get("messageStop").is_some() {
						// Message is complete, but we still wait for metadata
						continue;
					}

					// Handle metadata event (final event with usage)
					if let Some(metadata) = data.get("metadata") {
						self.done = true;

						// Capture usage if requested
						let captured_usage = if self.options.capture_usage {
							self.parse_usage(metadata).or_else(|| self.captured_data.usage.take())
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

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}

					// Log unhandled event types for debugging
					tracing::debug!("Unhandled Bedrock stream event: {:?}", data);
				}
				Some(Err(err)) => {
					tracing::error!("Bedrock stream error: {}", err);
					return Poll::Ready(Some(Err(Error::WebStream {
						model_iden: self.options.model_iden.clone(),
						cause: err.to_string(),
						error: err,
					})));
				}
				None => {
					// Stream ended without proper completion
					if !self.done {
						self.done = true;

						// Return what we have captured so far
						let inter_stream_end = InterStreamEnd {
							captured_usage: self.captured_data.usage.take(),
							captured_text_content: self.captured_data.content.take(),
							captured_reasoning_content: self.captured_data.reasoning_content.take(),
							captured_tool_calls: self.captured_data.tool_calls.take(),
							captured_thought_signatures: None,
						};

						return Poll::Ready(Some(Ok(InterStreamEvent::End(inter_stream_end))));
					}
					return Poll::Ready(None);
				}
			}
		}

		Poll::Pending
	}
}
